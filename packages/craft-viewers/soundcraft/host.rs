use std::{cell::RefCell, sync::Arc};
use serde_json::{json, Value};
use soundcraft_engine::{io, Engine};
use soundcraft_ui_egui::{Services, SoundApp};
use wasm_bindgen::prelude::*;

const MAX_BYTES: usize = 100 * 1024 * 1024;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftEmit)]
    fn emit(kind: &str, detail: &str, bytes: &[u8]);
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftTake)]
    fn take() -> JsValue;
}

thread_local! {
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    static OUTPUT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn request_repaint() {
    CONTEXT.with(|ctx| { if let Some(ctx) = ctx.borrow().as_ref() { ctx.request_repaint(); } });
}

fn event(kind: &str, detail: Value) { emit(kind, &detail.to_string(), &[]); }
fn field(value: &JsValue, name: &str) -> JsValue {
    js_sys::Reflect::get(value, &name.into()).unwrap_or(JsValue::UNDEFINED)
}

pub fn start() {
    io::set_deliver(|_, bytes| {
        if bytes.len() > MAX_BYTES { return Err("The exported audio exceeds 100 MiB.".into()); }
        OUTPUT.with(|out| *out.borrow_mut() = Some(bytes.to_vec()));
        Ok(())
    });
    wasm_bindgen_futures::spawn_local(async {
        match start_app().await {
            Ok(()) => event("ready", json!({})),
            Err(error) => event("error", json!({"error": error})),
        }
    });
}

async fn start_app() -> Result<(), String> {
    let canvas = web_sys::window().and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("craft_canvas"))
        .ok_or("The audio canvas is unavailable.")?
        .dyn_into::<web_sys::HtmlCanvasElement>().map_err(|_| "Invalid audio canvas.")?;
    let runner = eframe::WebRunner::new();
    runner.start(canvas, eframe::WebOptions::default(), Box::new(|cc| {
        CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(cc.egui_ctx.clone()));
        let engine = Engine::default();
        let player = soundcraft_playback::Player::new(Arc::new(engine.session().clone()));
        let services = Services { host_command: Some(Box::new(|id| {
            if id.starts_with("session.save") || id == "file.bounce_mix" || id == "file.export_midi" {
                event("save-request", json!({}));
                return Some(Ok(json!({"pending": "hostSave"})));
            }
            if id.starts_with("file.") || matches!(id, "session.new" | "session.open" | "session.close" | "app.quit") {
                return Some(Err("Use the file controls above the editor.".into()));
            }
            None
        })), ..Default::default() };
        Ok(Box::new(Host { app: SoundApp::new(engine, Some(player), services), opened: false,
            read_only: true, reported_dirty: false, last_exported: None }))
    })).await.map_err(|e| format!("The audio editor could not start: {e:?}"))?;
    std::mem::forget(runner);
    Ok(())
}

struct Host {
    app: SoundApp,
    opened: bool,
    read_only: bool,
    reported_dirty: bool,
    last_exported: Option<u64>,
}

impl Host {
    fn receive(&mut self, message: JsValue) -> Result<(), String> {
        match field(&message, "type").as_string().as_deref() {
            Some("open") => {
                if self.opened { return Err("An audio file is already open.".into()); }
                let name = field(&message, "name").as_string().ok_or("A file name is required.")?;
                let data = field(&message, "bytes");
                if !data.is_instance_of::<js_sys::ArrayBuffer>() { return Err("File bytes are required.".into()); }
                let data = js_sys::Uint8Array::new(&data);
                if data.length() == 0 || data.length() as usize > MAX_BYTES { return Err("Audio files must be between 1 byte and 100 MiB.".into()); }
                let bytes = data.to_vec();
                let ext = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
                match ext.as_str() {
                    "scraft" | "scraftz" => {
                        let missing = io::open_session_bytes(&mut self.app.engine, &name, &bytes).map_err(|e| e.to_string())?;
                        if !missing.is_empty() { return Err("This session references missing media. Open a bundled .scraftz session with its audio included.".into()); }
                    }
                    "mid" | "midi" | "smf" => { io::import_midi_bytes(&mut self.app.engine, &bytes, 0, true).map_err(|e| e.to_string())?; }
                    _ => { io::import_audio_bytes(&mut self.app.engine, &name, &bytes, None, None, 0).map_err(|e| e.to_string())?; }
                }
                self.app.engine.mark_clean();
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                self.opened = true;
                event("opened", json!({"revision": self.app.engine.revision, "readOnly": self.read_only}));
            }
            Some("set-read-only") => {
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                self.app.dialogs.open = None;
            }
            Some("export") => {
                if !self.opened || self.read_only { return Err("Enable editing before saving.".into()); }
                let request = field(&message, "requestId").as_string().ok_or("A save request identifier is required.")?;
                let format = field(&message, "format").as_string().ok_or("An export format is required.")?;
                let bytes = match format.as_str() {
                    "scraftz" => io::session_bundle(self.app.engine.session()).map_err(|e| e.to_string())?.0,
                    "mid" | "midi" | "smf" => io::export_midi(&self.app.engine, None).map_err(|e| e.to_string())?,
                    "wav" | "wave" | "bwf" | "aif" | "aiff" | "flac" => {
                        OUTPUT.with(|out| out.borrow_mut().take());
                        let encoding = match format.as_str() { "aif" | "aiff" => "aiff", "flac" => "flac", _ => "wav" };
                        let end = self.app.engine.session().content_end().max(1);
                        self.app.engine.execute("file.bounce_mix", &json!({"path": format!("output.{encoding}"), "format": encoding, "start": 0, "end": end, "bit_depth": "24", "dither": false})).map_err(|e| e.to_string())?;
                        OUTPUT.with(|out| out.borrow_mut().take()).ok_or("No audio was exported.")?
                    }
                    _ => return Err("This audio format cannot be exported.".into()),
                };
                if bytes.len() > MAX_BYTES { return Err("The exported file exceeds 100 MiB.".into()); }
                let revision = self.app.engine.revision;
                self.last_exported = Some(revision);
                emit("exported", &json!({"requestId": request, "revision": revision}).to_string(), &bytes);
            }
            Some("saved") => {
                if let Some(revision) = field(&message, "revision").as_f64() {
                    if self.last_exported == Some(revision as u64) && self.app.engine.revision as f64 == revision {
                        self.app.engine.mark_clean();
                        self.last_exported = None;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl eframe::App for Host {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        raw.dropped_files.clear();
        if self.read_only { raw.events.retain(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Cut)); }
        self.app.raw_input_hook(raw);
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        loop {
            let message = take();
            if message.is_null() || message.is_undefined() { break; }
            let request = field(&message, "requestId").as_string();
            if let Err(error) = self.receive(message) { event("error", json!({"error": error, "requestId": request})); }
        }
        self.app.logic(ctx);
        let dirty = self.opened && self.app.engine.is_dirty();
        if dirty != self.reported_dirty {
            self.reported_dirty = dirty;
            event("dirty", json!({"dirty": dirty, "revision": self.app.engine.revision}));
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.read_only {
            ui.horizontal(|ui| {
                ui.label("Flows Audio preview");
                if ui.button(if self.app.is_playing() { "Stop" } else { "Play" }).clicked() {
                    let command = if self.app.is_playing() { "transport.stop" } else { "transport.play" };
                    let _ = self.app.run(command, json!({}));
                }
            });
        }
        ui.add_enabled_ui(self.opened && !self.read_only, |ui| self.app.ui(ui));
    }
}
