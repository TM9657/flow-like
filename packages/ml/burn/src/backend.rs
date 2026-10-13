use crate::{BackendChoice, Error, Result};
use burn::tensor::Device;
use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
struct Resolved {
    backend: BackendChoice,
    /// Set when a higher-priority compiled backend failed its probe: when this one was chosen.
    fallback_at: Option<Instant>,
}
type ResolvedCache = HashMap<(BackendChoice, bool), Resolved>;
static RESOLVED: OnceLock<Mutex<ResolvedCache>> = OnceLock::new();
/// A fallback may stem from a GPU that was only briefly unavailable (a driver reset, or memory
/// another job held), so the preferred backends are probed again once it is this old.
const FALLBACK_RETRY: Duration = Duration::from_secs(60);

fn resolved_cache() -> MutexGuard<'static, ResolvedCache> {
    RESOLVED
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Select a usable backend. Auto prefers CUDA, ROCm, WGPU, then CPU.
/// Explicit choices remain strict.
pub fn resolve_backend(choice: &BackendChoice) -> Result<BackendChoice> {
    resolve_backend_with_fallback(choice, false)
}

/// Resolve an automatic or explicit preference by executing a numerical device probe.
/// Successful selections are cached for this process and reused while their device still
/// opens; a fallback is reused for at most a minute before the preferred backends are probed
/// again. Explicit GPU fallback moves through lower-priority GPU backends and then CPU; Auto
/// always permits this search.
pub fn resolve_backend_with_fallback(
    choice: &BackendChoice,
    allow_fallback: bool,
) -> Result<BackendChoice> {
    let require_physical_gpu = !matches!(
        choice,
        BackendChoice::Wgpu | BackendChoice::WgpuAdapter { .. }
    );
    resolve_cached(
        &mut resolved_cache(),
        choice,
        allow_fallback,
        &compiled_backends(),
        Instant::now(),
        |resolved| open_device(resolved, false).is_ok(),
        |candidate| probe_candidate(candidate, require_physical_gpu).map(|_| ()),
    )
}

/// A lost GPU (driver reset, unplugged or faulted device) stays unusable for the rest of the
/// process, so a cached selection whose device no longer opens is evicted and probed again.
fn resolve_cached(
    cache: &mut ResolvedCache,
    choice: &BackendChoice,
    allow_fallback: bool,
    compiled: &[&str],
    now: Instant,
    usable: impl FnOnce(&BackendChoice) -> bool,
    probe: impl FnMut(&BackendChoice) -> Result<()>,
) -> Result<BackendChoice> {
    let key = (choice.clone(), allow_fallback);
    if let Some(cached) = cache.get(&key).cloned() {
        let stale = cached
            .fallback_at
            .is_some_and(|chosen| now.saturating_duration_since(chosen) >= FALLBACK_RETRY);
        if !stale {
            if usable(&cached.backend) {
                return Ok(cached.backend);
            }
            forget_in(cache, &cached.backend);
        }
    }
    let resolved = resolve_with_probe(choice, allow_fallback, compiled, probe)?;
    let preferred = candidates(choice, allow_fallback)
        .into_iter()
        .find(|candidate| compiled.contains(&backend_name(candidate)));
    let fallback_at = (preferred.as_ref() != Some(&resolved)).then_some(now);
    cache.insert(
        (resolved.clone(), false),
        Resolved {
            backend: resolved.clone(),
            fallback_at: None,
        },
    );
    cache.insert(
        key,
        Resolved {
            backend: resolved.clone(),
            fallback_at,
        },
    );
    Ok(resolved)
}

fn forget_in(cache: &mut ResolvedCache, failed: &BackendChoice) {
    cache.retain(|_, resolved| resolved.backend != *failed);
}

/// Must not be called while the resolution cache is locked.
fn forget(failed: &BackendChoice) {
    forget_in(&mut resolved_cache(), failed);
}

/// Backends to try for `choice`, most preferred first.
fn candidates(choice: &BackendChoice, allow_fallback: bool) -> Vec<BackendChoice> {
    use BackendChoice::*;
    match (choice, allow_fallback) {
        (Auto, _) => vec![Cuda { device: 0 }, Rocm { device: 0 }, Wgpu, Cpu],
        (Cuda { device }, true) => vec![choice.clone(), Rocm { device: *device }, Wgpu, Cpu],
        (Rocm { .. }, true) => vec![choice.clone(), Wgpu, Cpu],
        (Wgpu | WgpuAdapter { .. }, true) => vec![choice.clone(), Cpu],
        _ => vec![choice.clone()],
    }
}

fn backend_name(candidate: &BackendChoice) -> &'static str {
    use BackendChoice::*;
    match candidate {
        Cpu => "cpu",
        Wgpu | WgpuAdapter { .. } => "wgpu",
        Cuda { .. } => "cuda",
        Rocm { .. } => "rocm",
        Auto => "auto",
    }
}

fn resolve_with_probe(
    choice: &BackendChoice,
    allow_fallback: bool,
    compiled: &[&str],
    mut probe: impl FnMut(&BackendChoice) -> Result<()>,
) -> Result<BackendChoice> {
    let mut failures = Vec::new();
    for candidate in candidates(choice, allow_fallback) {
        let name = backend_name(&candidate);
        if !compiled.contains(&name) {
            failures.push(format!("{candidate:?}: not enabled in this build"));
            continue;
        }
        match probe(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) => failures.push(format!("{candidate:?}: {error}")),
        }
    }
    Err(Error::Backend(format!(
        "No usable backend for {choice:?}: {}",
        failures.join("; ")
    )))
}

pub(crate) fn resolve_for_resume(
    requested: &BackendChoice,
    previous: Option<&BackendChoice>,
) -> Result<BackendChoice> {
    resolve_resume_with(requested, previous, resolve_backend)
}

fn resolve_resume_with(
    requested: &BackendChoice,
    previous: Option<&BackendChoice>,
    mut resolve: impl FnMut(&BackendChoice) -> Result<BackendChoice>,
) -> Result<BackendChoice> {
    if matches!(requested, BackendChoice::Auto) {
        if let Some(previous) = previous.filter(|previous| !matches!(previous, BackendChoice::Auto))
        {
            if let Ok(resolved) = resolve(previous) {
                return Ok(resolved);
            }
        }
    }
    resolve(requested)
}

/// Open a resolved backend. A failure evicts cached selections of that backend, so the next
/// resolution probes again instead of returning a lost GPU.
pub(crate) fn device(choice: &BackendChoice, training: bool) -> Result<Device> {
    open_device(choice, training).inspect_err(|_| forget(choice))
}

fn open_device(choice: &BackendChoice, training: bool) -> Result<Device> {
    #[cfg(windows)]
    exclude_working_directory_from_dll_search();
    let device = std::panic::catch_unwind(|| -> Result<Device> {
        #[cfg(ml_backend_wgpu)]
        if matches!(
            choice,
            BackendChoice::Wgpu | BackendChoice::WgpuAdapter { .. }
        ) && Device::enumerate(burn::tensor::DeviceType::Wgpu).is_empty()
        {
            return Err(Error::Backend("No WGPU adapter is available".into()));
        }
        let selected: Device = match choice {
            #[cfg(ml_backend_cpu)]
            BackendChoice::Cpu => Ok(Device::flex()),
            #[cfg(ml_backend_wgpu)]
            BackendChoice::Wgpu => Ok(Device::wgpu(Default::default())),
            #[cfg(ml_backend_wgpu)]
            BackendChoice::WgpuAdapter { kind, device } => {
                use crate::WgpuAdapterKind;
                use burn::tensor::DeviceKind;
                let kind = match kind {
                    WgpuAdapterKind::Discrete => DeviceKind::DiscreteGpu(*device),
                    WgpuAdapterKind::Integrated => DeviceKind::IntegratedGpu(*device),
                    WgpuAdapterKind::Virtual => DeviceKind::VirtualGpu(*device),
                };
                Ok(Device::wgpu(kind))
            }
            #[cfg(ml_backend_cuda)]
            BackendChoice::Cuda { device } => {
                cuda_driver_preflight()?;
                Ok(Device::cuda(*device))
            }
            #[cfg(ml_backend_rocm)]
            BackendChoice::Rocm { device } => {
                // HIP's generated C-ABI wrappers cannot unwind when a driver library is absent.
                crate::rocm_preflight::check()?;
                Ok(Device::rocm(*device))
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::Backend(format!(
                "{choice:?} is not enabled in this build"
            ))),
        }?;
        // Device constructors are lazy; allocate once so unavailable hardware returns a Result.
        let _ = burn::tensor::Tensor::<1>::zeros([1], &selected).into_data();
        selected
            .sync()
            .map_err(|error| Error::Backend(error.to_string()))?;
        Ok(selected)
    })
    .map_err(|_| {
        Error::Backend(format!(
            "Failed to initialize {choice:?}; verify the device and driver"
        ))
    })??;
    Ok(if training { device.autodiff() } else { device })
}

/// GPU runtimes load further libraries by bare name, which Windows also looks up in the
/// working directory. Removing it from this process's search order keeps a DLL planted there
/// from loading, while PATH still finds the CUDA toolkit libraries.
#[cfg(windows)]
fn exclude_working_directory_from_dll_search() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetDllDirectoryW(path: *const u16) -> i32;
    }
    static EXCLUDED: std::sync::Once = std::sync::Once::new();
    EXCLUDED.call_once(|| {
        let empty = [0u16];
        // SAFETY: `empty` is a nul-terminated UTF-16 string that outlives the call.
        unsafe { SetDllDirectoryW(empty.as_ptr()) };
    });
}

#[cfg(ml_backend_cuda)]
fn cuda_driver_preflight() -> Result<()> {
    static DRIVER: OnceLock<std::result::Result<libloading::Library, String>> = OnceLock::new();
    DRIVER
        .get_or_init(load_cuda_driver)
        .as_ref()
        .map(|_| ())
        .map_err(|error| Error::Backend(error.clone()))
}

/// The NVIDIA driver installs nvcuda.dll in System32. Only that directory is searched, so a
/// DLL planted in the working directory or on PATH is never loaded.
#[cfg(all(ml_backend_cuda, windows))]
fn load_cuda_driver() -> std::result::Result<libloading::Library, String> {
    use libloading::os::windows::{LOAD_LIBRARY_SEARCH_SYSTEM32, Library};
    unsafe { Library::load_with_flags("nvcuda.dll", LOAD_LIBRARY_SEARCH_SYSTEM32) }
        .map(libloading::Library::from)
        .map_err(|error| format!("CUDA driver library is unavailable in System32: {error}"))
}

#[cfg(all(ml_backend_cuda, not(windows)))]
fn load_cuda_driver() -> std::result::Result<libloading::Library, String> {
    let names: &[&str] = if cfg!(target_os = "macos") {
        &["libcuda.dylib", "libnvcuda.dylib"]
    } else {
        &[
            "libcuda.so",
            "libcuda.so.1",
            "libnvcuda.so",
            "libnvcuda.so.1",
        ]
    };
    for name in names {
        if let Ok(library) = unsafe { libloading::Library::new(name) } {
            return Ok(library);
        }
    }
    Err("CUDA driver library is unavailable".into())
}
/// Backends compiled into this binary. Hardware availability is checked when opening a session.
pub fn compiled_backends() -> Vec<&'static str> {
    let mut result = Vec::new();
    #[cfg(ml_backend_cpu)]
    result.push("cpu");
    #[cfg(ml_backend_wgpu)]
    result.push("wgpu");
    #[cfg(ml_backend_cuda)]
    result.push("cuda");
    #[cfg(ml_backend_rocm)]
    result.push("rocm");
    result
}

/// Execute a forward/backward calculation on the requested device without silently falling back.
pub fn probe_backend(choice: &BackendChoice) -> Result<crate::BackendProbe> {
    let choice = if matches!(choice, BackendChoice::Auto) {
        resolve_backend(choice)?
    } else {
        choice.clone()
    };
    probe_concrete(&choice).inspect_err(|_| forget(&choice))
}

fn probe_concrete(choice: &BackendChoice) -> Result<crate::BackendProbe> {
    probe_candidate(choice, false)
}

fn probe_candidate(
    choice: &BackendChoice,
    require_physical_gpu: bool,
) -> Result<crate::BackendProbe> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        use burn::tensor::Tensor;
        let device = open_device(choice, true)?;
        #[cfg(ml_backend_wgpu)]
        if require_physical_gpu
            && matches!(
                choice,
                BackendChoice::Wgpu | BackendChoice::WgpuAdapter { .. }
            )
            && device
                .identity()
                .is_none_or(|identity| identity.physical.is_none())
        {
            return Err(Error::Backend(
                "WGPU selected a software adapter; using the next backend".into(),
            ));
        }
        #[cfg(not(ml_backend_wgpu))]
        let _ = require_physical_gpu;
        let input = Tensor::<2>::from_floats([[1.0, 2.0], [3.0, 4.0]], &device).require_grad();
        let output = input.clone().square().sum();
        let value = output
            .clone()
            .into_data()
            .iter::<f32>()
            .next()
            .ok_or_else(|| Error::Backend("Probe produced no output".into()))?;
        let gradients = output.backward();
        let gradient = input
            .grad(&gradients)
            .ok_or_else(|| Error::Backend("Backend did not produce gradients".into()))?
            .into_data()
            .iter::<f32>()
            .collect::<Vec<_>>();
        device
            .sync()
            .map_err(|error| Error::Backend(error.to_string()))?;
        let mut maximum_error = (value - 30.0).abs();
        for (actual, expected) in gradient.iter().zip([2.0, 4.0, 6.0, 8.0]) {
            maximum_error = maximum_error.max((actual - expected).abs());
        }
        if gradient.len() != 4
            || !value.is_finite()
            || gradient.iter().any(|x| !x.is_finite())
            || !maximum_error.is_finite()
            || maximum_error > 1e-5
        {
            return Err(Error::Backend(format!(
                "Backend numerical probe failed: error={maximum_error}"
            )));
        }
        Ok(crate::BackendProbe {
            backend: choice.clone(),
            device: format!("{device:?}"),
            max_absolute_error: maximum_error,
        })
    }))
    .map_err(|_| {
        Error::Backend(format!(
            "{choice:?} panicked during its forward/backward probe"
        ))
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_resume_reuses_available_backend_and_migrates_when_unavailable() {
        let previous = BackendChoice::Cuda { device: 1 };
        let mut visited = Vec::new();
        let result = resolve_resume_with(&BackendChoice::Auto, Some(&previous), |choice| {
            visited.push(choice.clone());
            if matches!(choice, BackendChoice::Auto) {
                Ok(BackendChoice::Cpu)
            } else {
                Err(Error::Backend("GPU no longer present".into()))
            }
        })
        .unwrap();
        assert_eq!(result, BackendChoice::Cpu);
        assert_eq!(visited, vec![previous.clone(), BackendChoice::Auto]);
        assert_eq!(
            resolve_resume_with(&BackendChoice::Auto, Some(&previous), |choice| {
                assert_eq!(choice, &previous);
                Ok(previous.clone())
            })
            .unwrap(),
            previous
        );
        assert!(
            resolve_resume_with(&previous, Some(&BackendChoice::Cpu), |choice| {
                assert_eq!(choice, &previous);
                Err(Error::Backend("strict GPU unavailable".into()))
            })
            .is_err()
        );
    }

    #[test]
    fn automatic_selection_probes_in_priority_order_and_skips_disabled_backends() {
        let mut visited = Vec::new();
        let chosen = resolve_with_probe(
            &BackendChoice::Auto,
            false,
            &["cuda", "rocm", "wgpu", "cpu"],
            |candidate| {
                visited.push(candidate.clone());
                if matches!(candidate, BackendChoice::Wgpu) {
                    Ok(())
                } else {
                    Err(Error::Backend("missing driver".into()))
                }
            },
        )
        .unwrap();
        assert_eq!(chosen, BackendChoice::Wgpu);
        assert_eq!(
            visited,
            vec![
                BackendChoice::Cuda { device: 0 },
                BackendChoice::Rocm { device: 0 },
                BackendChoice::Wgpu
            ]
        );
        let chosen = resolve_with_probe(&BackendChoice::Auto, false, &["cpu"], |candidate| {
            assert_eq!(*candidate, BackendChoice::Cpu);
            Ok(())
        })
        .unwrap();
        assert_eq!(chosen, BackendChoice::Cpu);
    }

    #[test]
    fn strict_device_errors_and_fallback_preserves_requested_gpu_index() {
        let requested = BackendChoice::Cuda { device: 3 };
        let mut visited = Vec::new();
        assert!(
            resolve_with_probe(&requested, false, &["cuda", "cpu"], |candidate| {
                visited.push(candidate.clone());
                Err(Error::Backend("no such GPU".into()))
            })
            .is_err()
        );
        assert_eq!(visited, vec![requested.clone()]);
        visited.clear();
        let selected =
            resolve_with_probe(&requested, true, &["cuda", "rocm", "cpu"], |candidate| {
                visited.push(candidate.clone());
                if matches!(candidate, BackendChoice::Cpu) {
                    Ok(())
                } else {
                    Err(Error::Backend("no such GPU".into()))
                }
            })
            .unwrap();
        assert_eq!(selected, BackendChoice::Cpu);
        assert_eq!(
            visited,
            vec![
                requested,
                BackendChoice::Rocm { device: 3 },
                BackendChoice::Cpu
            ]
        );
    }

    const ALL_BACKENDS: &[&str] = &["cuda", "rocm", "wgpu", "cpu"];

    struct Machine {
        compiled: &'static [&'static str],
        gpu_alive: bool,
        now: Instant,
        probes: Vec<BackendChoice>,
    }
    impl Machine {
        fn new(compiled: &'static [&'static str]) -> Self {
            Self {
                compiled,
                gpu_alive: true,
                now: Instant::now(),
                probes: Vec::new(),
            }
        }
        fn resolve(
            &mut self,
            cache: &mut ResolvedCache,
            choice: &BackendChoice,
            allow_fallback: bool,
        ) -> Result<BackendChoice> {
            let gpu_alive = self.gpu_alive;
            let alive =
                move |candidate: &BackendChoice| gpu_alive || *candidate == BackendChoice::Cpu;
            let probes = &mut self.probes;
            resolve_cached(
                cache,
                choice,
                allow_fallback,
                self.compiled,
                self.now,
                alive,
                |candidate| {
                    probes.push(candidate.clone());
                    if alive(candidate) {
                        Ok(())
                    } else {
                        Err(Error::Backend("device lost".into()))
                    }
                },
            )
        }
    }

    fn cached(backend: BackendChoice) -> Resolved {
        Resolved {
            backend,
            fallback_at: None,
        }
    }

    #[test]
    fn cached_selection_is_probed_again_after_its_gpu_is_lost() {
        let cuda = BackendChoice::Cuda { device: 0 };
        let mut cache = ResolvedCache::new();
        let mut machine = Machine::new(ALL_BACKENDS);
        for _ in 0..2 {
            assert_eq!(
                machine
                    .resolve(&mut cache, &BackendChoice::Auto, true)
                    .unwrap(),
                cuda
            );
        }
        assert_eq!(machine.probes, vec![cuda.clone()]);
        machine.probes.clear();
        machine.gpu_alive = false;
        assert_eq!(
            machine
                .resolve(&mut cache, &BackendChoice::Auto, true)
                .unwrap(),
            BackendChoice::Cpu
        );
        assert_eq!(
            machine.probes,
            vec![
                cuda.clone(),
                BackendChoice::Rocm { device: 0 },
                BackendChoice::Wgpu,
                BackendChoice::Cpu
            ]
        );
        assert!(!cache.values().any(|resolved| resolved.backend == cuda));
        assert!(machine.resolve(&mut cache, &cuda, false).is_err());
        assert!(!cache.contains_key(&(cuda, false)));
    }

    #[test]
    fn fallback_selection_is_probed_again_once_the_gpu_may_have_recovered() {
        let cuda = BackendChoice::Cuda { device: 0 };
        let mut cache = ResolvedCache::new();
        let mut machine = Machine::new(ALL_BACKENDS);
        machine.gpu_alive = false;
        assert_eq!(
            machine
                .resolve(&mut cache, &BackendChoice::Auto, true)
                .unwrap(),
            BackendChoice::Cpu
        );
        assert_eq!(
            cache[&(BackendChoice::Auto, true)].fallback_at,
            Some(machine.now)
        );
        assert_eq!(cache[&(BackendChoice::Cpu, false)].fallback_at, None);
        machine.gpu_alive = true;
        machine.probes.clear();
        machine.now += FALLBACK_RETRY / 2;
        assert_eq!(
            machine
                .resolve(&mut cache, &BackendChoice::Auto, true)
                .unwrap(),
            BackendChoice::Cpu
        );
        assert!(machine.probes.is_empty());
        machine.now += FALLBACK_RETRY;
        assert_eq!(
            machine
                .resolve(&mut cache, &BackendChoice::Auto, true)
                .unwrap(),
            cuda
        );
        assert_eq!(machine.probes, vec![cuda.clone()]);
        assert_eq!(cache[&(BackendChoice::Auto, true)], cached(cuda));

        let mut cache = ResolvedCache::new();
        let mut cpu_only = Machine::new(&["cpu"]);
        cpu_only.gpu_alive = false;
        cpu_only
            .resolve(&mut cache, &BackendChoice::Auto, true)
            .unwrap();
        assert_eq!(
            cache[&(BackendChoice::Auto, true)],
            cached(BackendChoice::Cpu)
        );
        cpu_only.probes.clear();
        cpu_only.now += FALLBACK_RETRY * 2;
        cpu_only
            .resolve(&mut cache, &BackendChoice::Auto, true)
            .unwrap();
        assert!(cpu_only.probes.is_empty());
    }

    #[test]
    fn failed_device_evicts_every_selection_that_resolved_to_it() {
        let cuda = BackendChoice::Cuda { device: 0 };
        let mut cache = ResolvedCache::from([
            ((BackendChoice::Auto, true), cached(cuda.clone())),
            ((BackendChoice::Auto, false), cached(cuda.clone())),
            ((cuda.clone(), false), cached(cuda.clone())),
            ((BackendChoice::Wgpu, true), cached(BackendChoice::Cpu)),
        ]);
        forget_in(&mut cache, &cuda);
        assert_eq!(
            cache,
            ResolvedCache::from([((BackendChoice::Wgpu, true), cached(BackendChoice::Cpu))])
        );
    }
}
