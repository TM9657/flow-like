use std::sync::Arc;

use flow_like::{
    bit::{Bit, BitModelPreference},
    models::{
        history::{History, HistoryMessage, Role},
        llm::LLMCallback,
        response::Response,
    },
    state::FlowLikeEvent,
};
use tauri::AppHandle;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[tauri::command(async)]
pub async fn predict(
    app_handle: AppHandle,
    bit: Bit,
    id: String,
    system_prompt: String,
    prompt: String,
) -> Result<Response, TauriFunctionError> {
    println!("Invoking predict, prompt: {}", prompt);
    let model = {
        let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
        let model_factory = flow_like_state.lock().await.model_factory.clone();
        let mut model_factory = model_factory.lock().await;

        match model_factory.build(&bit, flow_like_state).await {
            Ok(model) => model,
            Err(e) => {
                return Err(TauriFunctionError::new(&format!(
                    "Error building model: {}",
                    e
                )))
            }
        }
    };

    let mut history = History::new("local".to_string(), vec![]);
    history.set_system_prompt(system_prompt.clone());
    history.push_message(HistoryMessage::from_string(Role::User, &prompt));
    history.set_stream(true);

    let sender = {
        let sender = TauriFlowLikeState::construct(&app_handle).await?;
        let sender = sender.lock().await;

        sender.event_sender.clone()
    };

    let callback: LLMCallback = Arc::new(move |response| {
        let callback_id = id.clone();
        let callback_sender = sender.clone();
        Box::pin({
            async move {
                let event = FlowLikeEvent::new(&format!("streaming_out:{}", callback_id), response);

                let _ = callback_sender.lock().await.send(event).await;
                Ok(())
            }
        })
    });

    let res = model.invoke(&history, Some(callback)).await?;

    Ok(res)
}

#[tauri::command(async)]
pub async fn find_best_model(
    app_handle: AppHandle,
    preferences: BitModelPreference,
    multimodal: bool,
    remote: bool,
) -> Result<Bit, TauriFunctionError> {
    let current_profile = TauriSettingsState::current_profile(&app_handle).await?;
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;

    let best_model = current_profile
        .hub_profile
        .get_best_model(&preferences, multimodal, remote, http_client)
        .await?;

    Ok(best_model)
}
