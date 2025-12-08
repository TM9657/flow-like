use crate::{bit::BitPack, state::FlowLikeState};
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_types::tokio::fs as async_fs;
use flow_like_types::{Result, anyhow};
use std::sync::Arc;

/// Ensures that local model weights are available before loading a model.
/// Attempts to redownload missing files, but falls back to cached weights when present.
pub async fn ensure_local_weights(
    pack: &BitPack,
    app_state: &Arc<FlowLikeState>,
    bit_id: &str,
    model_kind: &str,
) -> Result<()> {
    if is_pack_installed(pack, app_state).await {
        return Ok(());
    }

    if let Err(err) = pack.download(app_state.clone(), None).await {
        if is_pack_installed(pack, app_state).await {
            println!(
                "Failed to refresh {} {}. Using cached weights instead. Error: {}",
                model_kind, bit_id, err
            );
            return Ok(());
        }
        return Err(err);
    }

    if is_pack_installed(pack, app_state).await {
        return Ok(());
    }

    let missing = missing_local_artifacts(pack, app_state).await?;
    if missing.is_empty() {
        println!(
            "{} {} has cached files with metadata mismatches; continuing with local artifacts.",
            model_kind, bit_id
        );
        return Ok(());
    }

    Err(anyhow!(
        "Local cache for {} {} is unavailable. Missing artifacts: {}",
        model_kind,
        bit_id,
        missing.join(", ")
    ))
}

async fn is_pack_installed(pack: &BitPack, app_state: &Arc<FlowLikeState>) -> bool {
    pack.is_installed(app_state.clone()).await.unwrap_or(false)
}

async fn missing_local_artifacts(
    pack: &BitPack,
    app_state: &Arc<FlowLikeState>,
) -> Result<Vec<String>> {
    let store = FlowLikeState::bit_store(app_state).await?;
    let FlowLikeStore::Local(local_store) = store else {
        return Ok(pack
            .bits
            .iter()
            .map(|bit| format!("{} (no local store configured)", bit.id))
            .collect());
    };

    let mut missing = Vec::new();
    for bit in &pack.bits {
        let path = match bit.to_path(&local_store) {
            Some(path) => path,
            None => {
                missing.push(format!("{} (no path)", bit.id));
                continue;
            }
        };

        if !async_fs::try_exists(&path).await.unwrap_or(false) {
            missing.push(format!("{} ({})", bit.id, path.display()));
        }
    }

    Ok(missing)
}
