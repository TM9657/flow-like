use flow_like_types::json;
use tauri::{AppHandle, Url};

pub fn handle_deep_link(app_handle: &AppHandle, urls: &Vec<Url>) {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        if let Some(window) = app_handle.get_webview_window("main") {
            if !window.is_visible().unwrap_or(false) {
                let _ = window.show();
            }

            if window.is_minimized().unwrap_or(false) {
                let _ = window.unminimize();
            }

            let _ = window.set_focus();
        }
    }

    for url in urls {
        println!("Deep link URL: {}", url);

        // Handle file URLs first (iOS 'Open in...' / AirDrop of documents)
        if url.scheme() == "file" {
            // Convert to local path and notify UI to import
            if let Ok(path) = url.to_file_path() {
                let path_str = path.to_string_lossy().to_string();
                println!("Received file URL to import: {}", path_str);
                crate::utils::emit_throttled(
                    app_handle,
                    crate::utils::UiEmitTarget::All,
                    "import/file",
                    json::json!({ "path": path_str }),
                    std::time::Duration::from_millis(200),
                );
                continue;
            }
        }

        // Fallback: custom scheme deep links (e.g., flow-like://auth?...)
        let url_str = url.as_str();
        let command = url_str.trim_start_matches("flow-like://");

        let mut parts = command.splitn(2, '/');
        // we also need to split off any potential query parameters
        let command = parts.next().unwrap_or("");
        let mut parts = command.splitn(2, '?');

        match parts.next() {
            Some("auth") => {
                handle_auth(app_handle, url_str);
            }
            Some("trigger") => {
                handle_trigger(app_handle, url);
            }
            _ => {
                println!("Unknown deep link command: {}", command);
            }
        }
    }
}

fn handle_auth(app_handle: &AppHandle, url: &str) {
    println!("Handling auth URL: {}", url);
    crate::utils::emit_throttled(
        app_handle,
        crate::utils::UiEmitTarget::All,
        "oidc/url",
        json::json!({ "url": url }),
        std::time::Duration::from_millis(200),
    );
}

fn handle_trigger(app_handle: &AppHandle, url: &Url) {
    // Parse URL: flow-like://trigger/{app_id}/{...path}?param1=value1&param2=value2
    let path = url.path();

    // Remove leading slash and split into parts
    let path_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if path_parts.len() < 2 {
        println!("Invalid trigger URL: expected at least app_id and path");
        return;
    }

    let app_id = path_parts[0];
    let trigger_path = path_parts[1..].join("/");

    // Parse query parameters using Tauri's URL query method
    let query_params: serde_json::Value = if let Some(query) = url.query() {
        let mut params = serde_json::Map::new();
        // Parse query string manually
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                // URL decode the values
                let decoded_key = urlencoding::decode(key).unwrap_or_default().into_owned();
                let decoded_value = urlencoding::decode(value).unwrap_or_default().into_owned();
                params.insert(decoded_key, serde_json::Value::String(decoded_value));
            }
        }
        serde_json::Value::Object(params)
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    println!(
        "Trigger deep link: app_id='{}', path='{}', params={:?}",
        app_id, trigger_path, query_params
    );

    // Call the deeplink sink handler
    match crate::event_sink::deeplink::DeeplinkSink::handle_trigger(
        app_handle,
        app_id,
        &trigger_path,
        query_params,
    ) {
        Ok(true) => {
            println!("✅ Deeplink event triggered successfully");
        }
        Ok(false) => {
            println!("⚠️ Deeplink event not triggered (offline or not found)");
        }
        Err(e) => {
            println!("❌ Failed to trigger deeplink event: {}", e);
        }
    }
}
