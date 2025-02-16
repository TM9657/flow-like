mod functions;
mod profile;
mod settings;
mod state;
mod utils;
use flow_like::{
    state::{FlowLikeConfig, FlowLikeState},
    utils::{http::HTTPClient, local_object_store::LocalObjectStore},
};
use serde_json::Value;
use settings::Settings;
use state::TauriFlowLikeState;
use std::{borrow::Cow, sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::{DeepLinkExt, OpenUrlEvent};
use tokio::{sync::Mutex, time::interval};
use tracing_subscriber::prelude::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut settings_state = Settings::new();

    let bits_store = Arc::new(LocalObjectStore::new(settings_state.bit_dir.clone()).unwrap());
    let user_store = Arc::new(LocalObjectStore::new(settings_state.user_dir.clone()).unwrap());
    let project_store =
        Arc::new(LocalObjectStore::new(settings_state.project_dir.clone()).unwrap());
    let config: FlowLikeConfig = FlowLikeConfig::new(
        Some(bits_store.clone()),
        bits_store,
        user_store,
        project_store,
    );
    settings_state.set_config(&config);
    let settings_state = Arc::new(Mutex::new(settings_state));
    let (http_client, refetch_rx) = HTTPClient::new();
    let (state, _) = FlowLikeState::new(config, http_client);
    let state_ref = Arc::new(Mutex::new(state));

    let sentry_endpoint = std::option_env!("PUBLIC_SENTRY_ENDPOINT");
    let guard = sentry_endpoint.map(|endpoint| {
        sentry::init((
            endpoint,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                auto_session_tracking: true,
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });

    match guard {
        Some(_) => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(sentry_tracing::layer())
                .init();

            println!("Sentry Tracing Layer Initialized");
        }
        None => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .init();

            println!("Sentry Tracing Layer Not Initialized");
        }
    };

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let relay_handle = app.app_handle().clone();
            let gc_handle = relay_handle.clone();
            let refetch_handle = relay_handle.clone();
            let deep_link_handle = relay_handle.clone();

            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            app.deep_link().on_open_url(move |event| {
                let deep_link_handle = deep_link_handle.clone();
                handle_deep_link(&deep_link_handle, event);
            });

            tauri::async_runtime::spawn(async move {
                let handle = gc_handle;

                let model_factory = {
                    println!("Starting GC");
                    let flow_like_state = TauriFlowLikeState::construct(&handle).await.unwrap();
                    let flow_like_state = flow_like_state.lock().await;

                    flow_like_state.model_factory.clone()
                };
                println!("GC Started");

                let mut interval = interval(Duration::from_secs(1));

                loop {
                    interval.tick().await;

                    {
                        let mut state = model_factory.lock().await;
                        state.gc();
                    }
                }
            });

            tauri::async_runtime::spawn(async move {
                let handle = relay_handle;
                let buffer: Arc<dashmap::DashMap<Cow<'static, str>, Vec<Value>>> =
                    Arc::new(dashmap::DashMap::new());

                let mut receiver = {
                    println!("Starting Message Relay");
                    let flow_like_state = TauriFlowLikeState::construct(&handle).await.unwrap();
                    let mut flow_like_state = flow_like_state.lock().await;
                    flow_like_state.re_subscribe()
                };
                println!("Message Relay Started");

                let buffer_clone = Arc::clone(&buffer);
                tauri::async_runtime::spawn(async move {
                    loop {
                        if !buffer_clone.is_empty() {
                            buffer_clone.retain(|id, events: &mut Vec<Value>| {
                                if let Err(e) = handle.emit(id, &events) {
                                    eprintln!("Error sending event: {:?}", e);
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                });

                while let Some(event) = receiver.recv().await {
                    buffer
                        .entry(Cow::Owned(event.event_id.clone()))
                        .or_insert_with(|| Vec::with_capacity(10))
                        .push(event.payload);
                }
            });

            tauri::async_runtime::spawn(async move {
                let mut receiver = refetch_rx;
                let handle = refetch_handle;

                let http_client = {
                    println!("Starting Refetch Handler");
                    let flow_like_state = TauriFlowLikeState::construct(&handle).await.unwrap();
                    let flow_like_state = flow_like_state.lock().await;
                    flow_like_state.http_client.clone()
                };

                let client = http_client.client();

                println!("Refetch Handler Started");
                while let Some(event) = receiver.recv().await {
                    let request = event;
                    let request_hash = http_client.quick_hash(&request);
                    let response = match client.execute(request).await {
                        Ok(response) => response,
                        Err(e) => {
                            eprintln!("Error fetching request: {:?}", e);
                            continue;
                        }
                    };

                    let value = match response.json::<serde_json::Value>().await {
                        Ok(value) => value,
                        Err(e) => {
                            eprintln!("Error parsing response: {:?}", e);
                            continue;
                        }
                    };

                    match http_client.put(&request_hash, &value) {
                        Ok(result) => result,
                        Err(e) => {
                            eprintln!("Error putting value in cache: {:?}", e);
                            continue;
                        }
                    };
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state::TauriSettingsState(settings_state))
        .manage(state::TauriFlowLikeState(state_ref))
        .invoke_handler(tauri::generate_handler![
            functions::file::get_file_meta,
            functions::file::get_folder_meta,
            functions::ai::invoke::predict,
            functions::ai::invoke::find_best_model,
            functions::system::get_system_info,
            functions::download::init::init_downloads,
            functions::download::init::resume_download,
            functions::settings::profiles::get_profiles,
            functions::settings::profiles::get_default_profiles,
            functions::settings::profiles::get_current_profile,
            functions::settings::profiles::set_current_profile,
            functions::settings::profiles::upsert_profile,
            functions::settings::profiles::delete_profile,
            functions::settings::profiles::add_bit,
            functions::settings::profiles::remove_bit,
            functions::settings::profiles::get_bits_in_current_profile,
            functions::vault::vault_configured,
            functions::vault::create_vault_board,
            functions::vault::delete_vault_board,
            functions::vault::get_vault,
            functions::vault::get_vault_board,
            functions::vault::get_vault_boards,
            functions::vault::set_vault_config,
            functions::vault::get_vaults,
            functions::vault::get_vault_size,
            functions::vault::get_remote_vaults,
            functions::vault::create_vault,
            functions::vault::update_vault,
            functions::vault::delete_vault,
            functions::bit::get_bit_by_id,
            functions::bit::is_bit_installed,
            functions::bit::get_bit_size,
            functions::bit::get_pack_from_bit,
            functions::bit::get_bits_by_category,
            functions::bit::get_bits,
            functions::bit::download_bit,
            functions::bit::delete_bit,
            functions::bit::get_installed_bit,
            functions::flow::catalog::get_catalog,
            functions::flow::board::create_board,
            functions::flow::board::close_board,
            functions::flow::board::get_board,
            functions::flow::board::get_open_boards,
            functions::flow::board::update_board_meta,
            functions::flow::board::undo_board,
            functions::flow::board::redo_board,
            functions::flow::board::add_node_to_board,
            functions::flow::board::remove_node_from_board,
            functions::flow::board::move_node,
            functions::flow::board::upsert_comment,
            functions::flow::board::remove_comment,
            functions::flow::board::upsert_variable,
            functions::flow::board::remove_variable,
            functions::flow::board::connect_pins,
            functions::flow::board::disconnect_pins,
            functions::flow::board::upsert_pin,
            functions::flow::board::paste_nodes_to_board,
            functions::flow::board::update_node,
            functions::flow::board::save_board,
            functions::flow::run::create_run,
            functions::flow::run::execute_run,
            functions::flow::run::debug_step_run,
            functions::flow::run::get_run_status,
            functions::flow::run::get_run,
            functions::flow::run::get_run_traces,
            functions::flow::run::finalize_run,
        ]);

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(handle_instance));
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn handle_instance(app: &AppHandle, args: Vec<String>, _cwd: String) {
    let _ = app
        .get_webview_window("main")
        .expect("no main window")
        .set_focus();

    println!(
        "a new app instance was opened with {args:?} and the deep link event was already triggered"
    );
}

fn handle_deep_link(app: &AppHandle, event: OpenUrlEvent) {
    let _ = app
        .get_webview_window("main")
        .expect("no main window")
        .set_focus();

    println!("deep link URLs: {:?}", event.urls());
}
