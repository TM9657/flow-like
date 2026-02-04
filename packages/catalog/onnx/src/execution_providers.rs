//! Execution Provider Configuration for ONNX Runtime
//!
//! This module provides global initialization of ONNX Runtime's execution providers (EPs)
//! with automatic hardware detection and graceful fallback behavior.
//!
//! # Supported Execution Providers
//!
//! - **TensorRT**: NVIDIA GPUs with TensorRT (fastest for NVIDIA inference)
//! - **CUDA**: NVIDIA GPUs (requires CUDA toolkit)
//! - **CoreML**: Apple Neural Engine and GPU (macOS, iOS, tvOS)
//! - **DirectML**: Windows GPUs via DirectX 12 (AMD, Intel, NVIDIA)
//! - **XNNPACK**: Optimized CPU inference for ARM and x86 (great for mobile/edge)
//! - **CPU**: Always available fallback
//!
//! # Global Initialization
//!
//! Call `initialize_ort()` once at application startup, before creating any ONNX sessions.
//! This configures the global execution provider defaults that all sessions will use.
//!
//! ```ignore
//! // At app startup
//! flow_like_catalog_onnx::onnx::execution_providers::initialize_ort();
//!
//! // Later, all sessions automatically use the configured EPs
//! let session = Session::builder()?.commit_from_file("model.onnx")?;
//! ```
//!
//! # Graceful Fallback Behavior
//!
//! The initialization automatically:
//! 1. Detects available hardware acceleration
//! 2. Registers all available EPs in order of preference
//! 3. Falls back to CPU if no accelerators are available
//!
//! # Cross-Compilation Notes
//!
//! - All EP features compile on all platforms (they become no-ops where unsupported)
//! - CoreML only activates on Apple platforms at runtime
//! - DirectML only activates on Windows at runtime
//! - CUDA/TensorRT require the NVIDIA runtime on the target system
//! - XNNPACK works on all platforms with ARM or x86 CPUs

use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Track whether ORT has been initialized
static ORT_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Information about the active execution providers after initialization
#[derive(Debug, Clone, Default)]
pub struct ExecutionProviderInfo {
    /// List of active execution providers (in priority order)
    pub active_providers: Vec<String>,
    /// Whether any GPU/NPU acceleration is active
    pub accelerated: bool,
    /// Warnings during initialization
    pub warnings: Vec<String>,
}

/// Global EP info set during initialization (using RwLock for thread safety)
static EP_INFO: RwLock<Option<ExecutionProviderInfo>> = RwLock::new(None);

/// Initialize ONNX Runtime with the best available execution providers.
///
/// This function should be called once at application startup, before creating any ONNX sessions.
/// It's safe to call multiple times - subsequent calls are no-ops.
///
/// # Returns
///
/// Information about which execution providers were registered.
///
/// # Example
///
/// ```ignore
/// let info = initialize_ort();
/// println!("Active providers: {:?}", info.active_providers);
/// println!("GPU acceleration: {}", info.accelerated);
/// ```
pub fn initialize_ort() -> ExecutionProviderInfo {
    // Only initialize once
    if ORT_INITIALIZED.swap(true, Ordering::SeqCst) {
        // Already initialized, return cached info
        return EP_INFO
            .read()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_default();
    }

    let info = do_initialize_ort();

    // Cache the info
    if let Ok(mut guard) = EP_INFO.write() {
        *guard = Some(info.clone());
    }

    info
}

/// Check if ORT has been initialized
pub fn is_initialized() -> bool {
    ORT_INITIALIZED.load(Ordering::SeqCst)
}

/// Get the current execution provider info (returns None if not initialized)
pub fn get_ep_info() -> Option<ExecutionProviderInfo> {
    if !is_initialized() {
        return None;
    }
    EP_INFO.read().ok().and_then(|guard| guard.clone())
}

#[cfg(feature = "execute")]
fn do_initialize_ort() -> ExecutionProviderInfo {
    use flow_like_model_provider::ml::ort;
    use tracing::info;

    let mut active_providers = Vec::new();
    let warnings = Vec::new();
    let eps: Vec<ort::execution_providers::ExecutionProviderDispatch> = Vec::new();

    // Try to register EPs in order of preference
    // TensorRT > CUDA > CoreML > DirectML > XNNPACK > CPU

    // TensorRT (NVIDIA, fastest)
    #[cfg(feature = "tensorrt")]
    {
        if ort::execution_providers::TensorRTExecutionProvider::is_available() {
            info!("TensorRT execution provider available");
            eps.push(ort::execution_providers::TensorRTExecutionProvider::default().build());
            active_providers.push("TensorRT".to_string());
        } else {
            let msg = "TensorRT feature enabled but runtime not available";
            warn!("{}", msg);
            warnings.push(msg.to_string());
        }
    }

    // CUDA (NVIDIA)
    #[cfg(feature = "cuda")]
    {
        if ort::execution_providers::CUDAExecutionProvider::is_available() {
            info!("CUDA execution provider available");
            eps.push(ort::execution_providers::CUDAExecutionProvider::default().build());
            active_providers.push("CUDA".to_string());
        } else {
            let msg = "CUDA feature enabled but runtime not available";
            warn!("{}", msg);
            warnings.push(msg.to_string());
        }
    }

    // CoreML (Apple)
    #[cfg(feature = "coreml")]
    {
        if ort::execution_providers::CoreMLExecutionProvider::is_available() {
            info!("CoreML execution provider available");
            eps.push(ort::execution_providers::CoreMLExecutionProvider::default().build());
            active_providers.push("CoreML".to_string());
        } else {
            let msg = "CoreML feature enabled but not on Apple platform";
            warn!("{}", msg);
            warnings.push(msg.to_string());
        }
    }

    // DirectML (Windows)
    #[cfg(feature = "directml")]
    {
        if ort::execution_providers::DirectMLExecutionProvider::is_available() {
            info!("DirectML execution provider available");
            eps.push(ort::execution_providers::DirectMLExecutionProvider::default().build());
            active_providers.push("DirectML".to_string());
        } else {
            let msg = "DirectML feature enabled but not on Windows";
            warn!("{}", msg);
            warnings.push(msg.to_string());
        }
    }

    // XNNPACK (optimized CPU for ARM/x86)
    #[cfg(feature = "xnnpack")]
    {
        if ort::execution_providers::XNNPACKExecutionProvider::is_available() {
            info!("XNNPACK execution provider available");
            eps.push(ort::execution_providers::XNNPACKExecutionProvider::default().build());
            active_providers.push("XNNPACK".to_string());
        } else {
            let msg = "XNNPACK feature enabled but not available";
            warn!("{}", msg);
            warnings.push(msg.to_string());
        }
    }

    // CPU is always available as final fallback
    active_providers.push("CPU".to_string());

    let accelerated = active_providers.iter().any(|p| p != "CPU");

    // Initialize ORT with the collected execution providers
    if eps.is_empty() {
        info!("No GPU/NPU acceleration available, using CPU");
        ort::init().commit();
    } else {
        info!(
            "Initializing ORT with execution providers: {:?}",
            active_providers
        );
        ort::init().with_execution_providers(eps).commit();
    }

    ExecutionProviderInfo {
        active_providers,
        accelerated,
        warnings,
    }
}

#[cfg(not(feature = "execute"))]
fn do_initialize_ort() -> ExecutionProviderInfo {
    ExecutionProviderInfo {
        active_providers: vec!["CPU (execute feature disabled)".to_string()],
        accelerated: false,
        warnings: vec!["Execute feature not enabled".to_string()],
    }
}

/// Check availability of specific execution providers
#[cfg(feature = "execute")]
pub mod availability {
    #[allow(unused_imports)]
    use flow_like_model_provider::ml::ort;

    /// Check if CUDA is compiled in and available at runtime
    pub fn cuda_available() -> bool {
        #[cfg(feature = "cuda")]
        {
            ort::execution_providers::CUDAExecutionProvider::is_available()
        }
        #[cfg(not(feature = "cuda"))]
        {
            false
        }
    }

    /// Check if TensorRT is compiled in and available at runtime
    pub fn tensorrt_available() -> bool {
        #[cfg(feature = "tensorrt")]
        {
            ort::execution_providers::TensorRTExecutionProvider::is_available()
        }
        #[cfg(not(feature = "tensorrt"))]
        {
            false
        }
    }

    /// Check if CoreML is compiled in and available at runtime
    pub fn coreml_available() -> bool {
        #[cfg(feature = "coreml")]
        {
            ort::execution_providers::CoreMLExecutionProvider::is_available()
        }
        #[cfg(not(feature = "coreml"))]
        {
            false
        }
    }

    /// Check if DirectML is compiled in and available at runtime
    pub fn directml_available() -> bool {
        #[cfg(feature = "directml")]
        {
            ort::execution_providers::DirectMLExecutionProvider::is_available()
        }
        #[cfg(not(feature = "directml"))]
        {
            false
        }
    }

    /// Check if XNNPACK is compiled in and available at runtime
    pub fn xnnpack_available() -> bool {
        #[cfg(feature = "xnnpack")]
        {
            ort::execution_providers::XNNPACKExecutionProvider::is_available()
        }
        #[cfg(not(feature = "xnnpack"))]
        {
            false
        }
    }

    /// Get a summary of all available execution providers
    pub fn list_available() -> Vec<&'static str> {
        let mut available = Vec::new();
        if tensorrt_available() {
            available.push("TensorRT");
        }
        if cuda_available() {
            available.push("CUDA");
        }
        if coreml_available() {
            available.push("CoreML");
        }
        if directml_available() {
            available.push("DirectML");
        }
        if xnnpack_available() {
            available.push("XNNPACK");
        }
        available.push("CPU");
        available
    }
}

#[cfg(not(feature = "execute"))]
pub mod availability {
    /// Check if CUDA is compiled in and available at runtime
    pub fn cuda_available() -> bool {
        false
    }
    /// Check if TensorRT is compiled in and available at runtime
    pub fn tensorrt_available() -> bool {
        false
    }
    /// Check if CoreML is compiled in and available at runtime
    pub fn coreml_available() -> bool {
        false
    }
    /// Check if DirectML is compiled in and available at runtime
    pub fn directml_available() -> bool {
        false
    }
    /// Check if XNNPACK is compiled in and available at runtime
    pub fn xnnpack_available() -> bool {
        false
    }
    /// Get a summary of all available execution providers
    pub fn list_available() -> Vec<&'static str> {
        vec!["CPU (execute feature disabled)"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_idempotent() {
        // First call initializes
        let info1 = initialize_ort();
        // Second call returns cached info
        let info2 = initialize_ort();
        assert_eq!(info1.active_providers, info2.active_providers);
    }

    #[test]
    fn test_cpu_always_available() {
        let info = initialize_ort();
        assert!(info.active_providers.iter().any(|p| p.contains("CPU")));
    }
}
