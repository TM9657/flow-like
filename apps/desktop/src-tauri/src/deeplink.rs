use std::{collections::VecDeque, sync::{Arc, Mutex}};

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
        let url = url.as_str();
        let command = url.trim_start_matches("flow-like://");

        let mut parts = command.splitn(2, '/');
        // we also need to split off any potential query parameters
        let command = parts.next().unwrap_or("");
        let mut parts = command.splitn(2, '?');

        match parts.next() {
            Some("auth") => {
               handle_auth(&app_handle, url);
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