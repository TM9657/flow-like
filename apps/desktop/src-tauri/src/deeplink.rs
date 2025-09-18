
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
                    &app_handle,
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
                handle_auth(&app_handle, url_str);
            }
            _ => {}
        }
    }
}

fn handle_auth(app_handle: &AppHandle, url: &str) {
    println!("Handling auth URL: {}", url);
    crate::utils::emit_throttled(
        &app_handle,
        crate::utils::UiEmitTarget::All,
        "oidc/url",
        json::json!({ "url": url }),
        std::time::Duration::from_millis(200),
    );
}
