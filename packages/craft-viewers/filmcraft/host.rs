use filmcraft_engine::{Services, Session};
use filmcraft_ui_egui::FilmcraftApp;
use serde_json::{Value, json};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

const MAX_BYTES: usize = 200 * 1024 * 1024;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftEmit)]
    fn emit(kind: &str, detail: &str, bytes: &[u8]);
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftTake)]
    fn take() -> JsValue;
}

#[wasm_bindgen]
pub fn request_repaint() {
    crate::repaint();
}

fn event(kind: &str, detail: Value) {
    emit(kind, &detail.to_string(), &[]);
}
fn field(value: &JsValue, name: &str) -> JsValue {
    js_sys::Reflect::get(value, &name.into()).unwrap_or(JsValue::UNDEFINED)
}

fn host_command(id: &str) -> Option<Result<Value, String>> {
    if matches!(
        id,
        "file.save" | "file.saveAs" | "file.exportMedia" | "export.quick"
    ) {
        event("save-request", json!({}));
        return Some(Ok(json!({"pending": "hostSave"})));
    }
    if id.starts_with("help.")
        || id == "app.about"
        || id == "app.quit"
        || id.starts_with("export.queue.")
        || id.starts_with("file.")
            && !matches!(
                id,
                "file.newSequence"
                    | "file.newBin"
                    | "file.newAdjustmentLayer"
                    | "file.newColorMatte"
                    | "file.newBlackVideo"
                    | "file.newTransparentVideo"
                    | "file.newBarsAndTone"
                    | "file.newCountingLeader"
            )
    {
        return Some(Err("Use the file controls above the editor.".into()));
    }
    None
}

#[wasm_bindgen(start)]
pub fn start_host() {
    wasm_bindgen_futures::spawn_local(async {
        match start_app().await {
            Ok(()) => event("ready", json!({})),
            Err(error) => event("error", json!({"error": error})),
        }
    });
}

async fn start_app() -> Result<(), String> {
    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("craft_canvas"))
        .ok_or("The video canvas is unavailable.")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Invalid video canvas.")?;
    crate::webcodecs::probe().await;
    let runner = eframe::WebRunner::new();
    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| {
                crate::set_context(cc.egui_ctx.clone());
                let ctx = cc.egui_ctx.clone();
                crate::fs::set_on_data(move || ctx.request_repaint());
                crate::webcodecs::install();
                let mut session = Session::new(Arc::new(crate::fs::WebServices));
                session.host_command = Some(host_command);
                let mut app = FilmcraftApp::new(session);
                app.ui.mode = filmcraft_ui_egui::state::Mode::Edit;
                if let Some(rs) = cc.wgpu_render_state.clone() {
                    if rs.adapter.get_info().backend == eframe::wgpu::Backend::BrowserWebGpu {
                        app.set_wgpu(rs);
                    }
                }
                let audio = crate::audio::Handle::new();
                if let Some(audio) = &audio {
                    app.audio = Some(Box::new(audio.clone()));
                }
                Ok(Box::new(Host {
                    web: crate::WebApp {
                        app,
                        audio,
                        autosave: crate::recovery::Autosave::default(),
                    },
                    opened: false,
                    read_only: true,
                    reported_dirty: false,
                    last_exported: None,
                    pending: None,
                }))
            }),
        )
        .await
        .map_err(|e| format!("The video editor could not start: {e:?}"))?;
    std::mem::forget(runner);
    Ok(())
}

struct Pending {
    job: u64,
    request: String,
    revision: u64,
    path: String,
}
struct Host {
    web: crate::WebApp,
    opened: bool,
    read_only: bool,
    reported_dirty: bool,
    last_exported: Option<u64>,
    pending: Option<Pending>,
}

impl Host {
    fn execute(&mut self, id: &str, params: Value) -> Result<Value, String> {
        let hook = self.web.app.session.host_command.take();
        let result = self
            .web
            .app
            .session
            .execute(id, params)
            .map_err(|e| e.to_string());
        self.web.app.session.host_command = hook;
        result
    }

    fn receive(&mut self, message: JsValue) -> Result<(), String> {
        match field(&message, "type").as_string().as_deref() {
            Some("open") => {
                if self.opened {
                    return Err("A video file is already open.".into());
                }
                let name = field(&message, "name")
                    .as_string()
                    .ok_or("A file name is required.")?;
                let data = field(&message, "bytes");
                if !data.is_instance_of::<js_sys::ArrayBuffer>() {
                    return Err("File bytes are required.".into());
                }
                let data = js_sys::Uint8Array::new(&data);
                if data.length() == 0 || data.length() as usize > MAX_BYTES {
                    return Err("Video files must be between 1 byte and 200 MiB.".into());
                }
                let name: String = name
                    .chars()
                    .map(|c| if c == '/' || c == '\\' { '_' } else { c })
                    .collect();
                let path = format!("/files/{name}");
                crate::fs::put(&path, data.to_vec());
                let imported = self.execute("file.import", json!({"paths": [path]}))?;
                let errors = imported
                    .get("errors")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if !errors.is_empty() {
                    return Err(errors
                        .iter()
                        .map(|e| e.as_str().unwrap_or("Video import failed."))
                        .collect::<Vec<_>>()
                        .join("; "));
                }
                let item = imported
                    .get("items")
                    .and_then(Value::as_array)
                    .and_then(|v| v.first())
                    .and_then(Value::as_u64)
                    .ok_or("The video has no supported media stream.")?;
                self.execute("file.newSequence", json!({"name": name, "fromItem": item}))?;
                self.web.app.session.saved_revision = self.web.app.session.revision;
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                self.opened = true;
                event(
                    "opened",
                    json!({"revision": self.web.app.session.revision, "readOnly": self.read_only}),
                );
            }
            Some("set-read-only") => {
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true)
            }
            Some("export") => {
                if !self.opened || self.read_only {
                    return Err("Enable editing before saving.".into());
                }
                if self.pending.is_some() {
                    return Err("A video export is already in progress.".into());
                }
                if field(&message, "format").as_string().as_deref() != Some("mp4") {
                    return Err("Video exports use MP4.".into());
                }
                let request = field(&message, "requestId")
                    .as_string()
                    .ok_or("A save request identifier is required.")?;
                self.web.app.stop();
                let revision = self.web.app.session.revision;
                let path = format!("/exports/flows-{revision}.mp4");
                let result = self.execute("file.exportMedia", json!({"path": path, "format": "h264", "audio": true, "range": "entire", "wait": false}))?;
                let job = result
                    .get("job")
                    .and_then(Value::as_u64)
                    .ok_or("The video export did not start.")?;
                self.pending = Some(Pending {
                    job,
                    request,
                    revision,
                    path,
                });
            }
            Some("saved") => {
                if let Some(revision) = field(&message, "revision").as_f64() {
                    if self.last_exported == Some(revision as u64)
                        && self.web.app.session.revision as f64 == revision
                    {
                        self.web.app.session.saved_revision = revision as u64;
                        self.last_exported = None;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn poll_export(&mut self) {
        let Some(pending) = &self.pending else { return };
        let result = self
            .web
            .app
            .session
            .jobs
            .iter()
            .find(|j| j.id == pending.job)
            .and_then(|j| j.result.lock().unwrap_or_else(|e| e.into_inner()).clone());
        let Some(result) = result else { return };
        let Some(pending) = self.pending.take() else {
            return;
        };
        let offline = self
            .web
            .app
            .session
            .export_offline
            .get(&pending.job)
            .is_some_and(|media| !media.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
        if offline {
            event(
                "error",
                json!({"error": "Some media could not be decoded. The video was not saved.", "requestId": pending.request}),
            );
            return;
        }
        let bytes = result.and_then(|_| {
            crate::fs::WebServices
                .read_file(&pending.path)
                .map_err(|e| e.to_string())
        });
        match bytes {
            Ok(bytes) if bytes.len() <= MAX_BYTES && !bytes.is_empty() => {
                self.last_exported = Some(pending.revision);
                emit(
                    "exported",
                    &json!({"requestId": pending.request, "revision": pending.revision})
                        .to_string(),
                    &bytes,
                );
            }
            Ok(_) => event(
                "error",
                json!({"error": "The exported video is empty or exceeds 200 MiB.", "requestId": pending.request}),
            ),
            Err(error) => event(
                "error",
                json!({"error": error, "requestId": pending.request}),
            ),
        }
    }
}

impl eframe::App for Host {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        raw.dropped_files.clear();
        if self.read_only {
            raw.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { .. }
                        | egui::Event::Text(_)
                        | egui::Event::Paste(_)
                        | egui::Event::Cut
                )
            });
        }
        self.web.raw_input_hook(ctx, raw);
    }
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        loop {
            let message = take();
            if message.is_null() || message.is_undefined() {
                break;
            }
            let request = field(&message, "requestId").as_string();
            if let Err(error) = self.receive(message) {
                event("error", json!({"error": error, "requestId": request}));
            }
        }
        self.web.logic(ctx, frame);
        self.poll_export();
        let dirty = self.opened && self.web.app.session.is_dirty();
        if dirty != self.reported_dirty {
            self.reported_dirty = dirty;
            event(
                "dirty",
                json!({"dirty": dirty, "revision": self.web.app.session.revision}),
            );
        }
        if self.pending.is_some() {
            ctx.request_repaint();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.read_only {
            ui.horizontal(|ui| {
                ui.label("Flows Video preview");
                if ui
                    .button(if self.web.app.playback.playing {
                        "Stop"
                    } else {
                        "Play"
                    })
                    .clicked()
                {
                    self.web.app.toggle_play(1.0);
                }
            });
            ui.ctx().input_mut(|input| {
                input.events.clear();
                input.pointer = Default::default();
            });
        }
        ui.add_enabled_ui(self.opened && !self.read_only, |ui| self.web.ui(ui, frame));
    }
}
