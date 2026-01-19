//! WASM Engine configuration and management
//!
//! The engine is the core compilation and configuration unit for wasmtime.

use crate::error::{WasmError, WasmResult};
use crate::limits::WasmSecurityConfig;
use crate::module::WasmModule;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;
use wasmtime::{Cache, Config, Engine, OptLevel};

/// Configuration for the WASM engine
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Enable parallel compilation
    pub parallel_compilation: bool,
    /// Enable cranelift optimizations
    pub optimizations: bool,
    /// Optimization level
    pub opt_level: OptLevel,
    /// Enable fuel metering for execution limits
    pub fuel_metering: bool,
    /// Enable epoch interruption for timeouts
    pub epoch_interruption: bool,
    /// Enable memory growth
    pub memory_growth: bool,
    /// Cache directory for compiled modules (None = no caching)
    pub cache_dir: Option<std::path::PathBuf>,
    /// Default security configuration
    pub default_security: WasmSecurityConfig,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            parallel_compilation: true,
            optimizations: true,
            opt_level: OptLevel::Speed,
            fuel_metering: true,
            epoch_interruption: true,
            memory_growth: true,
            cache_dir: None,
            default_security: WasmSecurityConfig::default(),
        }
    }
}

impl WasmConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Development configuration (faster compilation, less optimization)
    pub fn development() -> Self {
        Self {
            parallel_compilation: true,
            optimizations: false,
            opt_level: OptLevel::None,
            fuel_metering: true,
            epoch_interruption: true,
            memory_growth: true,
            cache_dir: None,
            default_security: WasmSecurityConfig::default(),
        }
    }

    /// Production configuration (maximum optimization)
    pub fn production() -> Self {
        Self {
            parallel_compilation: true,
            optimizations: true,
            opt_level: OptLevel::Speed,
            fuel_metering: true,
            epoch_interruption: true,
            memory_growth: true,
            cache_dir: dirs_next::cache_dir().map(|p| p.join("flow-like").join("wasm")),
            default_security: WasmSecurityConfig::default(),
        }
    }

    pub fn with_cache_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    pub fn with_security(mut self, security: WasmSecurityConfig) -> Self {
        self.default_security = security;
        self
    }

    /// Build wasmtime Config from our config
    fn to_wasmtime_config(&self) -> WasmResult<Config> {
        let mut config = Config::new();

        // Compilation settings
        config.parallel_compilation(self.parallel_compilation);
        config.cranelift_opt_level(self.opt_level);

        // Runtime settings
        config.consume_fuel(self.fuel_metering);
        config.epoch_interruption(self.epoch_interruption);
        config.async_support(true);

        // Memory settings
        config.memory_init_cow(true);

        // Apply resource limits
        let limits = &self.default_security.limits;
        config.max_wasm_stack(limits.max_stack_depth as usize * 1024);

        // Caching
        if let Some(cache_dir) = &self.cache_dir {
            if let Err(e) = std::fs::create_dir_all(cache_dir) {
                tracing::warn!("Failed to create WASM cache directory: {}", e);
            } else {
                // Use wasmtime's built-in caching
                let cache_config_path = cache_dir.join("cache.toml");
                if !cache_config_path.exists() {
                    let cache_config = format!(
                        r#"[cache]
enabled = true
directory = "{}"
"#,
                        cache_dir.display()
                    );
                    if let Err(e) = std::fs::write(&cache_config_path, cache_config) {
                        tracing::warn!("Failed to write cache config: {}", e);
                    }
                }
                match Cache::from_file(Some(&cache_config_path)) {
                    Ok(cache) => {
                        config.cache(Some(cache));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load cache config: {}", e);
                    }
                }
            }
        }

        Ok(config)
    }
}

/// WASM Engine for compiling and running modules
pub struct WasmEngine {
    /// Wasmtime engine
    engine: Engine,
    /// Configuration
    config: WasmConfig,
    /// Cached compiled modules (hash -> module)
    module_cache: DashMap<String, Arc<WasmModule>>,
    /// Epoch ticker for timeouts
    epoch_ticker: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl WasmEngine {
    /// Create a new WASM engine with the given configuration
    pub fn new(config: WasmConfig) -> WasmResult<Self> {
        let wasmtime_config = config.to_wasmtime_config()?;
        let engine = Engine::new(&wasmtime_config).map_err(|e| {
            WasmError::compilation(format!("Failed to create wasmtime engine: {}", e))
        })?;

        Ok(Self {
            engine,
            config,
            module_cache: DashMap::new(),
            epoch_ticker: Arc::new(RwLock::new(None)),
        })
    }

    /// Create with default configuration
    pub fn default_engine() -> WasmResult<Self> {
        Self::new(WasmConfig::default())
    }

    /// Get reference to wasmtime engine
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get configuration
    pub fn config(&self) -> &WasmConfig {
        &self.config
    }

    /// Start the epoch ticker for timeout enforcement
    pub fn start_epoch_ticker(&self) {
        let mut ticker = self.epoch_ticker.write();
        if ticker.is_some() {
            return; // Already running
        }

        let engine = self.engine.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(10));
            loop {
                interval.tick().await;
                engine.increment_epoch();
            }
        });

        *ticker = Some(handle);
    }

    /// Stop the epoch ticker
    pub fn stop_epoch_ticker(&self) {
        let mut ticker = self.epoch_ticker.write();
        if let Some(handle) = ticker.take() {
            handle.abort();
        }
    }

    /// Load a module from bytes
    pub async fn load_module(&self, bytes: &[u8]) -> WasmResult<Arc<WasmModule>> {
        // Calculate hash for caching
        let hash = blake3::hash(bytes).to_hex().to_string();

        // Check cache
        if let Some(cached) = self.module_cache.get(&hash) {
            tracing::debug!("Using cached WASM module: {}", hash);
            return Ok(cached.clone());
        }

        // Compile module
        let module = WasmModule::from_bytes(self, bytes, hash.clone()).await?;
        let module = Arc::new(module);

        // Cache it
        self.module_cache.insert(hash, module.clone());

        Ok(module)
    }

    /// Load a module from file
    pub async fn load_module_from_file(
        &self,
        path: impl AsRef<Path>,
    ) -> WasmResult<Arc<WasmModule>> {
        let path = path.as_ref();
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|_e| WasmError::ModuleNotFound {
                path: path.display().to_string(),
            })?;

        self.load_module(&bytes).await
    }

    /// Load a module from URL
    #[cfg(feature = "http")]
    pub async fn load_module_from_url(&self, url: &str) -> WasmResult<Arc<WasmModule>> {
        let response: reqwest::Response =
            reqwest::get(url)
                .await
                .map_err(|_e| WasmError::ModuleNotFound {
                    path: url.to_string(),
                })?;

        if !response.status().is_success() {
            return Err(WasmError::ModuleNotFound {
                path: format!("{} (status: {})", url, response.status()),
            });
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| WasmError::Internal(format!("Failed to read module bytes: {}", e)))?;

        self.load_module(&bytes).await
    }

    /// Preload modules from a directory
    pub async fn preload_directory(
        &self,
        dir: impl AsRef<Path>,
    ) -> WasmResult<Vec<Arc<WasmModule>>> {
        let dir = dir.as_ref();
        let mut modules = Vec::new();

        let mut entries = tokio::fs::read_dir(dir).await.map_err(WasmError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(WasmError::Io)? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "wasm") {
                match self.load_module_from_file(&path).await {
                    Ok(module) => {
                        tracing::info!("Loaded WASM module: {}", path.display());
                        modules.push(module);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load WASM module {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(modules)
    }

    /// Clear the module cache
    pub fn clear_cache(&self) {
        self.module_cache.clear();
    }

    /// Get number of cached modules
    pub fn cached_module_count(&self) -> usize {
        self.module_cache.len()
    }

    /// Remove a specific module from cache
    pub fn evict_module(&self, hash: &str) -> bool {
        self.module_cache.remove(hash).is_some()
    }
}

impl Drop for WasmEngine {
    fn drop(&mut self) {
        self.stop_epoch_ticker();
    }
}

// Make WasmEngine Send + Sync safe
unsafe impl Send for WasmEngine {}
unsafe impl Sync for WasmEngine {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = WasmEngine::new(WasmConfig::development()).unwrap();
        assert_eq!(engine.cached_module_count(), 0);
    }

    #[test]
    fn test_config_development() {
        let config = WasmConfig::development();
        assert!(!config.optimizations);
        assert_eq!(config.opt_level, OptLevel::None);
    }

    #[test]
    fn test_config_production() {
        let config = WasmConfig::production();
        assert!(config.optimizations);
        assert_eq!(config.opt_level, OptLevel::Speed);
    }
}
