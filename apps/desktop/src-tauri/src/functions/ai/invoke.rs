use std::sync::Arc;

use flow_like::{
    bit::{Bit, BitModelPreference},
    flow_like_model_provider::{
        history::{History, HistoryMessage, Role},
        llm::LLMCallback,
        response::Response,
    },
    flow_like_types::intercom::{BufferedInterComHandler, InterComEvent},
};
use tauri::{AppHandle, Emitter};

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

    let buffered_sender = Arc::new(BufferedInterComHandler::new(
        Arc::new(move |event| {
            let app_handle = app_handle.clone();
            Box::pin({
                async move {
                    let first_event = event.first();
                    if let Some(first_event) = first_event {
                        if let Err(err) = app_handle.emit(&first_event.event_type, event.clone()) {
                            println!("Error emitting event: {}", err);
                        }
                    }
                    Ok(())
                }
            })
        }),
        Some(20),
        Some(100),
    ));

    let finalized = buffered_sender.clone();
    let callback: LLMCallback = Arc::new(move |response| {
        let callback_id = id.clone();
        let buffered_handler = buffered_sender.clone();
        Box::pin({
            async move {
                let handler = buffered_handler.clone();
                let event = InterComEvent::with_type(
                    &format!("streaming_out:{}", callback_id),
                    response.clone(),
                );

                handler.send(event).await?;
                Ok(())
            }
        })
    });

    let res = model.invoke(&history, Some(callback)).await?;

    finalized.flush().await?;

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
