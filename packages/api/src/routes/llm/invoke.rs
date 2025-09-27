use crate::{
    entity::{bit, template_profile},
    error::ApiError,
    middleware::jwt::AppUser,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
};
use flow_like::{
    bit::Bit,
    flow_like_model_provider::{
        history::{History, Usage},
        llm::{LLMCallback, ModelLogic, openai::OpenAIModel},
        response::Response,
        response_chunk::ResponseChunk,
    },
    profile::{Profile, Settings},
};
use flow_like_types::{
    Bytes, anyhow, bail,
    json::json,
    tokio::{self, sync::mpsc},
};
use futures_util::StreamExt;
use futures_util::stream::{self, Stream};
use sea_orm::EntityTrait;
use std::{convert::Infallible, sync::Arc, time::Duration};

enum StreamMsg {
    Progress(ResponseChunk),
    Error(String),
}

#[tracing::instrument(name = "POST /llm", skip(state, user))]
pub async fn invoke_llm(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(mut history): Json<History>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let sub = user.sub()?;
    let user_tier: flow_like::hub::UserTier = user.tier(&state).await?;

    let bit = bit::Entity::find_by_id(&history.model)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow!("Bit not found"))?;
    let bit = Bit::from(bit);
    let mut provider = bit
        .try_to_provider()
        .ok_or_else(|| anyhow!("Bit is not a model provider"))?;
    if &provider.provider_name != "hosted" {
        return Err(ApiError::BadRequest(
            "Only 'hosted' models are supported via this endpoint".into(),
        ));
    }

    let mut params = provider.params.clone().unwrap_or_default();

    let tier = if let Some(tier) = params.get("tier") {
        tier.as_str().unwrap_or("ENTERPRISE").to_string()
    } else {
        "ENTERPRISE".to_string()
    };

    user_tier
        .llm_tiers
        .iter()
        .find(|t| t == &&tier)
        .ok_or_else(|| {
            tracing::warn!(
                "User tier {:?} does not allow access to model {} tier",
                user_tier,
                tier
            );
            ApiError::Forbidden
        })?;

    params.insert(
        "api_key".into(),
        json!(std::env::var("OPENAI_API_KEY").unwrap_or_default()),
    );
    params.insert(
        "endpoint".into(),
        json!(std::env::var("OPENAI_ENDPOINT").unwrap_or_default()),
    );

    provider.params = Some(params);

    let model = OpenAIModel::from_params(&provider).await?;
    let tracking_id = user
        .tracking_id(&state)
        .await?
        .ok_or_else(|| anyhow!("User tracking ID not found"))?;
    history.user = Some(tracking_id);
    history.usage = Some(Usage { include: true });

    let (tx, rx) = mpsc::channel::<StreamMsg>(64);

    {
        let observer_tx: mpsc::Sender<StreamMsg> = tx.clone();
        tokio::spawn(async move {
            let callback: LLMCallback = Arc::new(move |response: ResponseChunk| {
                let tx = tx.clone();
                Box::pin({
                    async move {
                        if let Err(e) = tx.send(StreamMsg::Progress(response)).await {
                            tracing::error!("Error sending response chunk: {}", e);
                        }
                        Ok(())
                    }
                })
            });

            let mut invoke_fut = Box::pin(model.invoke(&history, Some(callback)));

            loop {
                tokio::select! {
                    _ = observer_tx.closed() => {
                        tracing::info!("Client disconnected, aborting model invocation");
                        break
                    },
                    _ = &mut invoke_fut => {
                        // Model invocation finished normally.
                    }
                }
            }
        });
    }

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(StreamMsg::Progress(p)) => {
                let data = serde_json::to_string(&p).unwrap_or_else(|_| "{}".into());
                Some((Ok(Event::default().event("data").data(data)), rx))
            }
            Some(StreamMsg::Error(msg)) => {
                let data = json!({"message": msg}).to_string();
                Some((Ok(Event::default().event("error").data(data)), rx))
            }
            None => None,
        }
    });

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .text("keep-alive")
            .interval(Duration::from_secs(15)),
    );
    Ok(sse)
}
