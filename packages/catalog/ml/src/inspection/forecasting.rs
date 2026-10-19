use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_catalog_core::FlowPath;
use flow_like_ml_forecast::{
    ForecastCapabilities, ForecastFamily, ForecastRequest, ForecastResult,
};
use flow_like_types::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod studio;
pub mod tables;
pub use tables::ForecastNode;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForecastModelHandle {
    /// Handle valid in the execution that loaded the checkpoint.
    pub cache_key: String,
    pub family: ForecastFamily,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub bit_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadForecastModelRequest {
    pub family: ForecastFamily,
    /// Folder containing upstream config.json and model.safetensors, or the shard index and files.
    pub directory: FlowPath,
    #[serde(default = "default_maximum_bytes")]
    pub maximum_bytes: u64,
}

fn default_maximum_bytes() -> u64 {
    2 * 1024 * 1024 * 1024
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadHuggingFaceForecastRequest {
    pub family: ForecastFamily,
    /// Public Hugging Face model repository, such as mistralai/Timestral-209M.
    pub repository: String,
    /// Branch, tag or commit. The loader resolves it to an immutable commit before downloading.
    #[serde(default = "default_revision")]
    pub revision: String,
    #[serde(default = "default_maximum_bytes")]
    pub maximum_bytes: u64,
}

fn default_revision() -> String {
    "main".into()
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForecastNodeRequest {
    pub model: ForecastModelHandle,
    pub forecast: ForecastRequest,
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
struct LoadedForecastModel {
    model: std::sync::Mutex<flow_like_ml_forecast::ForecastModel>,
    capabilities: ForecastCapabilities,
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
type SharedModel = std::sync::Arc<LoadedForecastModel>;

#[cfg(any(feature = "inference", feature = "forecasting"))]
fn shared_model(model: flow_like_ml_forecast::ForecastModel) -> SharedModel {
    std::sync::Arc::new(LoadedForecastModel {
        capabilities: model.capabilities(),
        model: std::sync::Mutex::new(model),
    })
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
pub(super) async fn cache_model(
    context: &ExecutionContext,
    loaded: flow_like_ml_forecast::ForecastModel,
    handle: ForecastModelHandle,
) -> Result<ForecastModelHandle> {
    if loaded.capabilities().family != handle.family {
        return Err(flow_like_types::anyhow!(
            "Forecast model handle has the wrong family"
        ));
    }
    super::published::run_map::<SharedModel>(context, "ml.forecast.models")
        .await?
        .insert(handle.cache_key.clone(), shared_model(loaded), 4);
    Ok(handle)
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
async fn forecast_values(
    context: &ExecutionContext,
    handle: &ForecastModelHandle,
    request: ForecastRequest,
) -> Result<ForecastResult> {
    request.validate()?;
    let model = model(context, handle).await?;
    tokio::task::spawn_blocking(move || -> Result<_> {
        Ok(model
            .model
            .lock()
            .map_err(|_| flow_like_types::anyhow!("Forecast model lock was poisoned"))?
            .forecast(&request)?)
    })
    .await?
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
async fn model(context: &ExecutionContext, handle: &ForecastModelHandle) -> Result<SharedModel> {
    let cache = super::published::run_map::<SharedModel>(context, "ml.forecast.models").await?;
    let model = cache.get(&handle.cache_key).ok_or_else(|| {
        flow_like_types::anyhow!(
            "Forecast model is no longer loaded in this execution. Run its load node again"
        )
    })?;
    let family = model.capabilities.family;
    if family != handle.family {
        return Err(flow_like_types::anyhow!(
            "Forecast model handle has the wrong family"
        ));
    }
    Ok(model)
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
fn child(directory: &FlowPath, filename: &str) -> Result<FlowPath> {
    flow_like_ml_forecast::weights::validate_checkpoint_filename(filename)?;
    Ok(FlowPath::new(
        flow_like_storage::join_object_path(&directory.object_path(), filename).to_string(),
        directory.store_ref.clone(),
        directory.cache_store_ref.clone(),
    ))
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
async fn copy_file(
    context: &mut ExecutionContext,
    source: &FlowPath,
    destination: &std::path::Path,
    maximum: u64,
    optional: bool,
) -> Result<Option<u64>> {
    use flow_like_storage::object_store::ObjectStoreExt;
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    let store = source.to_store(context).await?;
    let result = flow_like_catalog_core::found_or_missing(
        store.as_generic().get(&source.object_path()).await,
        &source.path,
        &source.store_ref,
    )?;
    let Some(result) = result else {
        if optional {
            return Ok(None);
        }
        return Err(flow_like_types::anyhow!(
            "Checkpoint file {} was not found",
            source.path
        ));
    };
    if result.meta.size > maximum {
        return Err(flow_like_types::anyhow!(
            "Checkpoint file {} exceeds the remaining {maximum} byte limit",
            source.path
        ));
    }
    let mut file = tokio::fs::File::create(destination).await?;
    let mut stream = result.into_stream();
    let mut count = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        count = count
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| flow_like_types::anyhow!("Checkpoint size overflows"))?;
        if count > maximum {
            return Err(flow_like_types::anyhow!(
                "Checkpoint file {} exceeds {maximum} bytes",
                source.path
            ));
        }
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(Some(count))
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
async fn load_model(
    context: &mut ExecutionContext,
    request: LoadForecastModelRequest,
) -> Result<ForecastModelHandle> {
    if request.maximum_bytes == 0 {
        return Err(flow_like_types::anyhow!(
            "Checkpoint byte limit must be positive"
        ));
    }
    let temporary = tempfile::tempdir()?;
    let mut remaining = request.maximum_bytes;
    let config = "config.json";
    let count = copy_file(
        context,
        &child(&request.directory, config)?,
        &temporary.path().join(config),
        remaining.min(2 * 1024 * 1024),
        false,
    )
    .await?
    .unwrap_or(0);
    remaining -= count;
    let index = "model.safetensors.index.json";
    let count = copy_file(
        context,
        &child(&request.directory, index)?,
        &temporary.path().join(index),
        remaining.min(8 * 1024 * 1024),
        true,
    )
    .await?;
    let files = if let Some(count) = count {
        remaining -= count;
        #[derive(Deserialize)]
        struct Index {
            weight_map: std::collections::BTreeMap<String, String>,
        }
        let value: Index = flow_like_types::json::from_slice(
            &tokio::fs::read(temporary.path().join(index)).await?,
        )?;
        let files: std::collections::BTreeSet<String> = value.weight_map.into_values().collect();
        if files.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Checkpoint index contains no weights"
            ));
        }
        files
    } else {
        std::collections::BTreeSet::from(["model.safetensors".to_string()])
    };
    for file in files {
        if !file.ends_with(".safetensors") {
            return Err(flow_like_types::anyhow!(
                "Checkpoint shard must use SafeTensors: {file}"
            ));
        }
        let source = child(&request.directory, &file)?;
        let count = copy_file(
            context,
            &source,
            &temporary.path().join(&file),
            remaining,
            false,
        )
        .await?
        .unwrap_or(0);
        remaining -= count;
    }
    let family = request.family;
    let loaded = tokio::task::spawn_blocking(move || {
        flow_like_ml_forecast::load_cpu(family, temporary.path())
    })
    .await??;
    let handle = ForecastModelHandle {
        cache_key: flow_like_types::create_id(),
        family,
        repository: None,
        revision: None,
        bit_id: None,
    };
    cache_model(context, loaded, handle).await
}

#[cfg(any(any(feature = "inference", feature = "forecasting"), test))]
fn hub_url(repository: &str, route: &[&str]) -> Result<flow_like_types::reqwest::Url> {
    let parts: Vec<_> = repository.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || part.contains("..")
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err(flow_like_types::anyhow!(
            "Hugging Face repository must have the form owner/model"
        ));
    }
    let mut url = flow_like_types::reqwest::Url::parse("https://huggingface.co")?;
    let mut path = url
        .path_segments_mut()
        .map_err(|_| flow_like_types::anyhow!("Invalid Hub URL"))?;
    if route.first() == Some(&"api") {
        path.extend(["api", "models", parts[0], parts[1]]);
        path.extend(&route[1..]);
    } else {
        path.extend(parts);
        path.extend(route);
    }
    drop(path);
    Ok(url)
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
async fn download_hub_file(
    context: &ExecutionContext,
    repository: &str,
    revision: &str,
    filename: &str,
    destination: &std::path::Path,
    maximum: u64,
) -> Result<u64> {
    use tokio::io::AsyncWriteExt;
    flow_like_ml_forecast::weights::validate_checkpoint_filename(filename)?;
    let url = hub_url(repository, &["resolve", revision, filename])?;
    let mut response = context
        .app_state
        .http_client
        .client()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    if response.content_length().is_some_and(|size| size > maximum) {
        return Err(flow_like_types::anyhow!(
            "Checkpoint file {filename} exceeds the remaining {maximum} byte limit"
        ));
    }
    let mut file = tokio::fs::File::create(destination).await?;
    let mut bytes = 0_u64;
    while let Some(chunk) = response.chunk().await? {
        bytes = bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| flow_like_types::anyhow!("Checkpoint size overflows"))?;
        if bytes > maximum {
            return Err(flow_like_types::anyhow!(
                "Checkpoint file {filename} exceeds {maximum} bytes"
            ));
        }
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(bytes)
}

#[cfg(any(feature = "inference", feature = "forecasting"))]
async fn load_huggingface(
    context: &mut ExecutionContext,
    request: LoadHuggingFaceForecastRequest,
) -> Result<ForecastModelHandle> {
    if request.maximum_bytes == 0 || request.revision.trim().is_empty() {
        return Err(flow_like_types::anyhow!(
            "Checkpoint byte limit must be positive and revision must be nonempty"
        ));
    }
    #[derive(Deserialize)]
    struct Metadata {
        sha: String,
        siblings: Vec<Sibling>,
    }
    #[derive(Deserialize)]
    struct Sibling {
        rfilename: String,
    }
    let url = hub_url(&request.repository, &["api", "revision", &request.revision])?;
    let mut response = context
        .app_state
        .http_client
        .client()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err(flow_like_types::anyhow!(
                "Hugging Face metadata exceeds 4 MiB"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    let metadata: Metadata = flow_like_types::json::from_slice(&body)?;
    if metadata.sha.len() != 40 || !metadata.sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(flow_like_types::anyhow!(
            "Hugging Face returned an invalid checkpoint commit"
        ));
    }
    let names: std::collections::BTreeSet<_> = metadata
        .siblings
        .into_iter()
        .map(|file| file.rfilename)
        .collect();
    if !names.contains("config.json") {
        return Err(flow_like_types::anyhow!(
            "Repository contains no config.json"
        ));
    }
    let temporary = tempfile::tempdir()?;
    let mut remaining = request.maximum_bytes;
    remaining -= download_hub_file(
        context,
        &request.repository,
        &metadata.sha,
        "config.json",
        &temporary.path().join("config.json"),
        remaining.min(2 * 1024 * 1024),
    )
    .await?;
    let index = "model.safetensors.index.json";
    let files = if names.contains(index) {
        remaining -= download_hub_file(
            context,
            &request.repository,
            &metadata.sha,
            index,
            &temporary.path().join(index),
            remaining.min(8 * 1024 * 1024),
        )
        .await?;
        #[derive(Deserialize)]
        struct Index {
            weight_map: std::collections::BTreeMap<String, String>,
        }
        let index: Index = flow_like_types::json::from_slice(
            &tokio::fs::read(temporary.path().join(index)).await?,
        )?;
        index
            .weight_map
            .into_values()
            .collect::<std::collections::BTreeSet<_>>()
    } else {
        std::collections::BTreeSet::from(["model.safetensors".to_string()])
    };
    if files.is_empty() {
        return Err(flow_like_types::anyhow!("Checkpoint has no weights"));
    }
    for name in files {
        if !name.ends_with(".safetensors") || !names.contains(&name) {
            return Err(flow_like_types::anyhow!(
                "Missing SafeTensors checkpoint file {name}"
            ));
        }
        flow_like_ml_forecast::weights::validate_checkpoint_filename(&name)?;
        remaining -= download_hub_file(
            context,
            &request.repository,
            &metadata.sha,
            &name,
            &temporary.path().join(&name),
            remaining,
        )
        .await?;
    }
    let family = request.family;
    let loaded = tokio::task::spawn_blocking(move || {
        flow_like_ml_forecast::load_cpu(family, temporary.path())
    })
    .await??;
    let handle = ForecastModelHandle {
        cache_key: flow_like_types::create_id(),
        family,
        repository: Some(request.repository),
        revision: Some(metadata.sha),
        bit_id: None,
    };
    cache_model(context, loaded, handle).await
}

#[crate::register_node]
#[derive(Default)]
pub struct LoadHuggingFaceForecastNode;

#[flow_like_types::async_trait]
impl NodeLogic for LoadHuggingFaceForecastNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<LoadHuggingFaceForecastRequest, ForecastModelHandle>(
            "ml_load_huggingface_forecast_model",
            "Load Hugging Face Forecast Model",
            "Download a supported public forecasting checkpoint at a pinned revision and load it for native Rust inference",
            "loadHuggingFaceForecastModel",
            "AI/ML/Forecasting",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(any(feature = "inference", feature = "forecasting"))]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request = context.evaluate_pin("request").await?;
            let result = load_huggingface(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(any(feature = "inference", feature = "forecasting")))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_urls_pin_downloads_and_encode_revisions() {
        assert_eq!(
            hub_url("amazon/chronos-2", &["api", "revision", "refs/pr/1"])
                .unwrap()
                .as_str(),
            "https://huggingface.co/api/models/amazon/chronos-2/revision/refs%2Fpr%2F1"
        );
        assert_eq!(
            hub_url(
                "mistralai/Timestral-5M",
                &["resolve", "012345", "model.safetensors"]
            )
            .unwrap()
            .as_str(),
            "https://huggingface.co/mistralai/Timestral-5M/resolve/012345/model.safetensors"
        );
        for repository in [
            "../private",
            "owner/../private",
            "https://example.com/model",
            "owner/model?token=x",
            "owner/%2f",
        ] {
            assert!(hub_url(repository, &["resolve", "main", "config.json"]).is_err());
        }
    }

    #[test]
    fn forecast_nodes_register_without_an_inference_runtime() {
        let ids: std::collections::BTreeSet<_> = crate::get_catalog()
            .into_iter()
            .map(|node| node.get_node().name)
            .collect();
        for name in [
            "ml_load_forecasting_bit",
            "ml_load_forecast_model",
            "ml_load_huggingface_forecast_model",
            "ml_forecast",
            "ml_forecast_values",
            "ml_forecast_model_info",
        ] {
            assert!(ids.contains(name), "Missing forecast node {name}");
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct LoadForecastModelNode;

#[flow_like_types::async_trait]
impl NodeLogic for LoadForecastModelNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<LoadForecastModelRequest, ForecastModelHandle>(
            "ml_load_forecast_model",
            "Load Forecast Model",
            "Load a supported SafeTensors forecasting checkpoint from storage for native Rust inference",
            "loadForecastModel",
            "AI/ML/Forecasting",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(any(feature = "inference", feature = "forecasting"))]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request = context.evaluate_pin("request").await?;
            let result = load_model(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(any(feature = "inference", feature = "forecasting")))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ForecastValuesNode;

#[flow_like_types::async_trait]
impl NodeLogic for ForecastValuesNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ForecastNodeRequest, ForecastResult>(
            "ml_forecast_values",
            "Forecast Values",
            "Forecast from arrays of equally spaced historical targets and optional numeric covariates",
            "forecastValues",
            "AI/ML/Forecasting",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(any(feature = "inference", feature = "forecasting"))]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ForecastNodeRequest = context.evaluate_pin("request").await?;
            let result = forecast_values(context, &request.model, request.forecast).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(any(feature = "inference", feature = "forecasting")))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ForecastModelInfoNode;

#[flow_like_types::async_trait]
impl NodeLogic for ForecastModelInfoNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ForecastModelHandle, ForecastCapabilities>(
            "ml_forecast_model_info",
            "Forecast Model Info",
            "Read the loaded checkpoint's context, horizon, quantile and covariate capabilities",
            "forecastModelInfo",
            "AI/ML/Forecasting",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(any(feature = "inference", feature = "forecasting"))]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request = context.evaluate_pin("request").await?;
            let model = model(context, &request).await?;
            let result = model.capabilities.clone();
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(any(feature = "inference", feature = "forecasting")))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}
