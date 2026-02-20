//! AOT (Ahead-of-Time) compilation cache for WASM modules
//!
//! **Local-only cache** — every installation compiles from `.wasm` source.
//! We NEVER accept external `.cwasm` files. A `.wasm` is sandboxed; a `.cwasm`
//! is raw native code. The only `.cwasm` files that enter this cache are ones
//! we compiled ourselves from verified `.wasm` bytecode.
//!
//! Each cached artifact is keyed by:
//! - blake3 hash of the original `.wasm` bytes  (content identity)
//! - OS + architecture                          (platform identity)
//! - wasmtime major version                     (compiler identity)
//!
//! A `.b3` sidecar stores a plain blake3 checksum to detect disk corruption.

use crate::error::WasmResult;
use std::path::{Path, PathBuf};
use wasmtime::Module;

#[cfg(feature = "component-model")]
use wasmtime::component::Component;

const WASMTIME_VERSION: &str = "40";

fn cache_key(wasm_hash: &str) -> String {
    format!(
        "{}-{}-{}-wt{}",
        wasm_hash,
        std::env::consts::OS,
        std::env::consts::ARCH,
        WASMTIME_VERSION,
    )
}

pub struct AotCache {
    modules_dir: PathBuf,
    #[cfg(feature = "component-model")]
    components_dir: PathBuf,
}

impl AotCache {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let base = cache_dir.into();
        Self {
            modules_dir: base.join("modules"),
            #[cfg(feature = "component-model")]
            components_dir: base.join("components"),
        }
    }

    fn artifact_path(dir: &Path, wasm_hash: &str) -> PathBuf {
        dir.join(format!("{}.cwasm", cache_key(wasm_hash)))
    }

    fn checksum_path(dir: &Path, wasm_hash: &str) -> PathBuf {
        dir.join(format!("{}.cwasm.b3", cache_key(wasm_hash)))
    }

    fn load_and_verify(dir: &Path, wasm_hash: &str) -> Option<Vec<u8>> {
        let artifact = Self::artifact_path(dir, wasm_hash);
        let checksum_file = Self::checksum_path(dir, wasm_hash);

        let serialized = std::fs::read(&artifact).ok()?;
        let expected = std::fs::read_to_string(&checksum_file).ok()?;

        let actual = blake3::hash(&serialized).to_hex().to_string();
        if expected.trim() != actual {
            tracing::warn!("AOT cache corrupted for {}, discarding", wasm_hash);
            let _ = std::fs::remove_file(&artifact);
            let _ = std::fs::remove_file(&checksum_file);
            return None;
        }

        Some(serialized)
    }

    fn write_artifact(dir: &Path, wasm_hash: &str, serialized: &[u8]) -> WasmResult<()> {
        std::fs::create_dir_all(dir)?;

        let artifact = Self::artifact_path(dir, wasm_hash);
        let checksum_file = Self::checksum_path(dir, wasm_hash);

        std::fs::write(&artifact, serialized)?;
        std::fs::write(
            &checksum_file,
            blake3::hash(serialized).to_hex().to_string(),
        )?;

        tracing::info!(
            "Saved AOT cache: {} ({} bytes)",
            artifact.display(),
            serialized.len()
        );
        Ok(())
    }

    fn evict(dir: &Path, wasm_hash: &str) {
        let _ = std::fs::remove_file(Self::artifact_path(dir, wasm_hash));
        let _ = std::fs::remove_file(Self::checksum_path(dir, wasm_hash));
    }

    /// Try to load a precompiled module. Returns `None` on cache miss or corruption.
    ///
    /// # Safety
    /// `Module::deserialize` loads native machine code. This is safe here because
    /// only self-compiled artifacts from verified `.wasm` bytecode enter this cache.
    pub fn load_module(&self, engine: &wasmtime::Engine, wasm_hash: &str) -> Option<Module> {
        let serialized = Self::load_and_verify(&self.modules_dir, wasm_hash)?;

        // SAFETY: only self-compiled artifacts; checksum guards against corruption
        match unsafe { Module::deserialize(engine, &serialized) } {
            Ok(module) => {
                tracing::debug!("AOT cache hit for module {}", wasm_hash);
                Some(module)
            }
            Err(e) => {
                tracing::warn!("AOT deserialize failed for module {}: {}", wasm_hash, e);
                Self::evict(&self.modules_dir, wasm_hash);
                None
            }
        }
    }

    pub fn save_module(&self, module: &Module, wasm_hash: &str) {
        match module.serialize() {
            Ok(s) => {
                if let Err(e) = Self::write_artifact(&self.modules_dir, wasm_hash, &s) {
                    tracing::warn!("Failed to save AOT module {}: {}", wasm_hash, e);
                }
            }
            Err(e) => tracing::warn!("Failed to serialize module {}: {}", wasm_hash, e),
        }
    }

    #[cfg(feature = "component-model")]
    pub fn load_component(&self, engine: &wasmtime::Engine, wasm_hash: &str) -> Option<Component> {
        let serialized = Self::load_and_verify(&self.components_dir, wasm_hash)?;

        // SAFETY: same guarantees as load_module
        match unsafe { Component::deserialize(engine, &serialized) } {
            Ok(component) => {
                tracing::debug!("AOT cache hit for component {}", wasm_hash);
                Some(component)
            }
            Err(e) => {
                tracing::warn!("AOT deserialize failed for component {}: {}", wasm_hash, e);
                Self::evict(&self.components_dir, wasm_hash);
                None
            }
        }
    }

    #[cfg(feature = "component-model")]
    pub fn save_component(&self, component: &Component, wasm_hash: &str) {
        match component.serialize() {
            Ok(s) => {
                if let Err(e) = Self::write_artifact(&self.components_dir, wasm_hash, &s) {
                    tracing::warn!("Failed to save AOT component {}: {}", wasm_hash, e);
                }
            }
            Err(e) => tracing::warn!("Failed to serialize component {}: {}", wasm_hash, e),
        }
    }

    pub fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.modules_dir);
        #[cfg(feature = "component-model")]
        let _ = std::fs::remove_dir_all(&self.components_dir);
    }
}
