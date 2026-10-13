use crate::{Error, Result};
use libloading::Library;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

// Keep the checked runtime loaded. HIP's generated extern-C wrappers abort if a symbol is absent.
static LIBRARIES: OnceLock<std::result::Result<(Library, Library), String>> = OnceLock::new();

pub(crate) fn check() -> Result<()> {
    LIBRARIES
        .get_or_init(load)
        .as_ref()
        .map(|_| ())
        .map_err(|error| Error::Backend(error.clone()))
}

fn load() -> std::result::Result<(Library, Library), String> {
    let hip = load_library("amdhip64")?;
    let hiprtc = load_library("hiprtc")?;
    for &name in REQUIRED_SYMBOLS {
        let library = if name.starts_with("hiprtc") {
            &hiprtc
        } else {
            &hip
        };
        // Resolve addresses only; no foreign code is called by this check.
        unsafe { library.get::<unsafe extern "C" fn()>(name.as_bytes()) }
            .map_err(|error| format!("ROCm runtime is missing {name}: {error}"))?;
    }
    Ok((hip, hiprtc))
}

fn load_library(name: &str) -> std::result::Result<Library, String> {
    let roots = ["ROCM_PATH", "HIP_PATH"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    for candidate in candidates(name, &roots) {
        if let Ok(library) = unsafe { open(&candidate) } {
            return Ok(library);
        }
    }
    Err(format!("ROCm runtime library {name} is unavailable"))
}

/// Follow the search order of the pinned cubecl-hip-sys dynamic loader: ROCm roots, then the
/// bare name, whose later load there reuses the module loaded here. On Windows the bare name
/// never reaches PATH, so the HIP SDK's bin directories take its place.
fn candidates(name: &str, roots: &[PathBuf]) -> Vec<PathBuf> {
    let names = if cfg!(target_os = "windows") {
        vec![format!("{name}.dll")]
    } else if cfg!(target_os = "macos") {
        vec![format!("lib{name}.dylib")]
    } else {
        vec![
            format!("lib{name}.so"),
            format!("lib{name}.so.1"),
            format!("lib{name}.so.0"),
        ]
    };
    let in_directory = |directory: PathBuf| names.iter().map(move |name| directory.join(name));
    let sdk_roots: &[PathBuf] = if cfg!(windows) { roots } else { &[] };
    roots
        .iter()
        .flat_map(|root| [root.join("lib"), root.clone()])
        .flat_map(in_directory)
        .chain(names.iter().map(PathBuf::from))
        .chain(
            sdk_roots
                .iter()
                .map(|root| root.join("bin"))
                .flat_map(in_directory),
        )
        .collect()
}

#[cfg(not(windows))]
unsafe fn open(candidate: &Path) -> std::result::Result<Library, libloading::Error> {
    unsafe { Library::new(candidate) }
}

/// Neither the runtime nor its dependencies are resolved from the working directory or PATH.
#[cfg(windows)]
unsafe fn open(candidate: &Path) -> std::result::Result<Library, libloading::Error> {
    unsafe { libloading::os::windows::Library::load_with_flags(candidate, load_flags(candidate)) }
        .map(Library::from)
}

#[cfg(windows)]
fn load_flags(candidate: &Path) -> u32 {
    use libloading::os::windows::{
        LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    };
    if candidate.is_absolute() {
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
    } else {
        LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
    }
}

// Symbols invoked by the pinned CubeCL 0.11 HIP runtime, including graph and allocation paths.
const REQUIRED_SYMBOLS: &[&str] = &[
    "hipDeviceGetPCIBusId",
    "hipEventCreateWithFlags",
    "hipEventDestroy",
    "hipEventElapsedTime",
    "hipEventRecord",
    "hipEventSynchronize",
    "hipFree",
    "hipFreeHost",
    "hipGetDeviceCount",
    "hipGetDevicePropertiesR0600",
    "hipGraphDestroy",
    "hipGraphExecDestroy",
    "hipGraphGetNodes",
    "hipGraphInstantiate",
    "hipGraphLaunch",
    "hipGraphNodeGetType",
    "hipGraphUpload",
    "hipHostMalloc",
    "hipMalloc",
    "hipMemGetInfo",
    "hipMemcpy2DAsync",
    "hipMemcpyDtoDAsync",
    "hipMemcpyDtoHAsync",
    "hipMemcpyHtoDAsync",
    "hipModuleGetFunction",
    "hipModuleLaunchKernel",
    "hipModuleLoadData",
    "hipSetDevice",
    "hipSetDeviceFlags",
    "hipStreamBeginCapture",
    "hipStreamCreateWithFlags",
    "hipStreamDestroy",
    "hipStreamEndCapture",
    "hipStreamSynchronize",
    "hipStreamWaitEvent",
    "hiprtcCompileProgram",
    "hiprtcCreateProgram",
    "hiprtcDestroyProgram",
    "hiprtcGetCode",
    "hiprtcGetCodeSize",
    "hiprtcGetProgramLog",
    "hiprtcGetProgramLogSize",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_is_searched_under_rocm_roots_then_by_bare_name_then_sdk_bin() {
        let root = PathBuf::from("rocm");
        let order = candidates("amdhip64", std::slice::from_ref(&root))
            .iter()
            .map(|path| {
                if path.parent() == Some(Path::new("")) {
                    1
                } else if path.starts_with(root.join("bin")) {
                    2
                } else {
                    0
                }
            })
            .collect::<Vec<_>>();
        assert!(order.is_sorted());
        assert!(order.contains(&0) && order.contains(&1));
        assert_eq!(order.contains(&2), cfg!(windows));
    }

    #[cfg(windows)]
    #[test]
    fn windows_loads_never_search_the_working_directory() {
        use libloading::os::windows::{
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        };
        assert_eq!(
            load_flags(Path::new("amdhip64.dll")),
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
        );
        assert_eq!(
            load_flags(Path::new(r"C:\ROCm\bin\amdhip64.dll")),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
        );
    }
}
