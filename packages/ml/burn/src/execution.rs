use crate::{CancellationToken, Error, Result};
use burn::{
    module::{Module, ModuleVisitor, Param},
    tensor::{Bool, ExecutionError, Int, Tensor, TensorData},
};
use std::sync::{Mutex, MutexGuard, TryLockError};

// Burn's device RNG is shared across workers and model families in this process.
// A phase includes seeding and every random operation that depends on that seed.
static RNG: Mutex<()> = Mutex::new(());
const DEVICE_ERROR_CHARACTERS: usize = 2048;
const OUT_OF_MEMORY_MARKERS: [&str; 7] = [
    "out of memory",
    "out-of-memory",
    "out_of_memory",
    "outofmemory",
    "out of device memory",
    "out_of_device_memory",
    "can't allocate buffer",
];

pub(crate) fn lock(cancellation: Option<&CancellationToken>) -> Result<MutexGuard<'static, ()>> {
    let Some(cancellation) = cancellation else {
        return Ok(RNG.lock().unwrap_or_else(|error| error.into_inner()));
    };
    loop {
        if cancellation.is_cancelled() {
            return Err(Error::Cancelled);
        }
        match RNG.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(error)) => return Ok(error.into_inner()),
            Err(TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
        }
    }
}

/// Map a failed device read to an error instead of a panic. Backends report allocation
/// failures at the next synchronization, so reads are where out-of-memory surfaces.
pub(crate) fn device_error(error: impl std::fmt::Display) -> Error {
    let text = error.to_string();
    let lower = text.to_ascii_lowercase();
    let detail = match text.char_indices().nth(DEVICE_ERROR_CHARACTERS) {
        Some((end, _)) => &text[..end],
        None => text.as_str(),
    };
    if OUT_OF_MEMORY_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        Error::Backend(format!(
            "out of device memory; reduce the batch size or input resolution: {detail}"
        ))
    } else {
        Error::Backend(format!("device read failed: {detail}"))
    }
}

/// Read every parameter once. Lazy initializers draw from the device RNG on first access, and
/// loaded weights are read from their file on first access, so neither depends on later state.
pub(crate) fn materialize(model: &impl Module) -> Result<()> {
    struct Materialize(Result<()>);
    impl Materialize {
        fn read(&mut self, read: impl FnOnce() -> std::result::Result<TensorData, ExecutionError>) {
            if self.0.is_ok()
                && let Err(error) = read()
            {
                self.0 = Err(device_error(error));
            }
        }
    }
    impl ModuleVisitor for Materialize {
        fn visit_float<const D: usize>(&mut self, param: &Param<Tensor<D>>) {
            self.read(|| param.val().try_into_data());
        }
        fn visit_int<const D: usize>(&mut self, param: &Param<Tensor<D, Int>>) {
            self.read(|| param.val().try_into_data());
        }
        fn visit_bool<const D: usize>(&mut self, param: &Param<Tensor<D, Bool>>) {
            self.read(|| param.val().try_into_data());
        }
    }
    let mut visitor = Materialize(Ok(()));
    model.visit(&mut visitor);
    visitor.0
}

#[cfg(all(test, feature = "cpu"))]
mod tests {
    use super::*;
    #[test]
    fn cancellation_interrupts_a_waiting_rng_phase() {
        let _test = crate::tests::TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let guard = lock(None).unwrap();
        let cancellation = CancellationToken::new();
        let worker_cancel = cancellation.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let cancelled = matches!(lock(Some(&worker_cancel)), Err(Error::Cancelled));
            tx.send(cancelled).unwrap();
        });
        cancellation.cancel();
        assert!(rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap());
        drop(guard);
        worker.join().unwrap();
    }

    #[test]
    fn device_errors_name_out_of_memory_and_stay_bounded() {
        for reason in [
            "out of device memory allocating 1073741824 bytes",
            "An out-of-memory error happened during launch",
            "CUDA_ERROR_OUT_OF_MEMORY",
            "hipErrorOutOfMemory",
            "ERROR_OUT_OF_DEVICE_MEMORY",
        ] {
            let error = device_error(ExecutionError::WithContext {
                reason: reason.into(),
            });
            assert!(matches!(error, Error::Backend(_)));
            let message = error.to_string();
            assert!(message.contains("out of device memory"), "{message}");
            assert!(message.contains(reason), "{message}");
        }
        let other = device_error("kernel compilation failed").to_string();
        assert!(other.contains("device read failed: kernel compilation failed"));
        assert!(!other.contains("out of device memory"));
        let long = device_error("x".repeat(10 * DEVICE_ERROR_CHARACTERS)).to_string();
        assert!(long.len() < 2 * DEVICE_ERROR_CHARACTERS);
    }

    #[derive(Module, Debug)]
    struct Lazy {
        weight: Param<Tensor<2>>,
    }

    #[test]
    fn materialize_reads_every_parameter_of_a_lazy_module() {
        let device = burn::tensor::Device::flex();
        let module = Lazy {
            weight: burn::nn::Initializer::Ones.init([2, 3], &device),
        };
        assert!(!module.weight.is_initialized());
        materialize(&module).unwrap();
        assert!(module.weight.is_initialized());
    }
}
