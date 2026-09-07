use crate::error::CompilerError;
use flow_like_wasm::limits::WasmLimits;
use flow_like_wasm::manifest::PackageNodeEntry;
use flow_like_wasm::{WasmConfig, WasmEngine, WasmSecurityConfig};

/// Allow the main module, shim, WASI adapter, and fixup used by componentized
/// packages. The adapter shares the main memory; the fixup shares the shim table.
pub(crate) fn metadata_security() -> WasmSecurityConfig {
    let limits = WasmLimits {
        max_instances: 4,
        // Existing packages need up to 84.5 MiB of initial linear memory and
        // 14,915 entries in a function table before get_nodes can run.
        memory_limit: 128 * 1024 * 1024,
        max_table_elements: 20_000,
        ..WasmLimits::restrictive()
    };
    WasmSecurityConfig::restrictive()
        .with_limits(limits)
        .for_metadata()
}

/// Extract node definitions using the compiler's bounded metadata sandbox.
/// This runs the package's metadata exports without granting host capabilities.
pub async fn extract_nodes(wasm_bytes: &[u8]) -> Result<Vec<PackageNodeEntry>, CompilerError> {
    let security = metadata_security();
    let engine = WasmEngine::new(
        WasmConfig::default()
            .without_cache()
            .with_security(security.clone()),
    )
    .map_err(|e| CompilerError::Compilation(format!("Host engine creation failed: {e}")))?;
    let loaded = engine.load_auto(wasm_bytes).await.map_err(|e| {
        CompilerError::Compilation(format!("Failed to load WASM for node extraction: {e}"))
    })?;
    engine.start_epoch_ticker();
    let mut instance = loaded.instantiate(&engine, security).await.map_err(|e| {
        CompilerError::Compilation(format!(
            "Failed to instantiate WASM for node extraction: {e}"
        ))
    })?;
    let defs = instance
        .call_get_nodes()
        .await
        .map_err(|e| CompilerError::Compilation(format!("Failed to call get_nodes: {e}")))?;
    Ok(defs
        .iter()
        .map(flow_like_wasm::definition_to_package_entry)
        .collect())
}

#[cfg(test)]
#[path = "metadata_tests.rs"]
mod tests;
