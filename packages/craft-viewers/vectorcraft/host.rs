use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::json;
use vectorcraft_engine::{Session, cmd::fileio};
use vectorcraft_ui_egui::{Services, VectorcraftApp};
use wasm_bindgen::prelude::*;

const MAX_BYTES: usize = 50 * 1024 * 1024;
const INPUTS: &[&str] = &["svg", "svgz", "ai", "ait", "eps", "dxf", "emf", "wmf", "vectorcraft", "drawcraft", "vctemplate", "af", "afdesign", "afpub"];

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftEmit)]
    fn emit(kind: &str, detail: &str, bytes: &[u8]);
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftTake)]
    fn take() -> JsValue;
}

thread_local! {
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn request_repaint() {
    CONTEXT.with(|ctx| {
        if let Some(ctx) = ctx.borrow().as_ref() { ctx.request_repaint(); }
    });
}

fn event(kind: &str, detail: serde_json::Value) { emit(kind, &detail.to_string(), &[]); }
fn field(value: &JsValue, name: &str) -> JsValue {
    js_sys::Reflect::get(value, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED)
}
fn extension(name: &str) -> String { name.rsplit('.').next().unwrap_or("").to_ascii_lowercase() }

fn host_command(id: &str) -> Option<Result<serde_json::Value, String>> {
    if matches!(id, "file.save" | "file.saveAs" | "document.save" | "document.saveAs") {
        event("save-request", json!({}));
        return Some(Ok(json!({"pending": "hostSave"})));
    }
    if id.starts_with("file.") || id.starts_with("document.open") || id.starts_with("document.save")
        || id.starts_with("document.export") || id.starts_with("document.close")
        || matches!(id, "document.new" | "document.revert" | "ui.home" | "ui.discord" | "app.home" | "app.about" | "app.quit")
    {
        return Some(Err("Use the file controls above the editor.".into()));
    }
    None
}

pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Warn).ok();
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
        .ok_or("The vector canvas is unavailable.")?
        .dyn_into::<web_sys::HtmlCanvasElement>().map_err(|_| "Invalid vector canvas.")?;
    eframe::WebRunner::new().start(canvas, eframe::WebOptions::default(), Box::new(|cc| {
        CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(cc.egui_ctx.clone()));
        let services = Services { host_command: Some(Box::new(host_command)), ..Default::default() };
        Ok(Box::new(Host {
            app: VectorcraftApp::new(Session::new(), services),
            opened: false, read_only: true, name: String::new(), reported_dirty: false,
            last_exported: None, preview_revision: 0, textures: HashMap::new(),
            renderer: vectorcraft_render::Renderer::new(),
        }))
    })).await.map_err(|e| format!("Flows Vector could not start: {e:?}"))
}

struct Host {
    app: VectorcraftApp,
    opened: bool,
    read_only: bool,
    name: String,
    reported_dirty: bool,
    last_exported: Option<u64>,
    preview_revision: u64,
    textures: HashMap<usize, egui::TextureHandle>,
    renderer: vectorcraft_render::Renderer,
}

impl Host {
    fn receive(&mut self, message: JsValue) -> Result<(), String> {
        match field(&message, "type").as_string().as_deref() {
            Some("open") => {
                if self.opened { return Err("This editor already has a file open.".into()); }
                let name = field(&message, "name").as_string().ok_or("A file name is required.")?;
                if !INPUTS.contains(&extension(&name).as_str()) { return Err("This vector format is unsupported.".into()); }
                let data = field(&message, "bytes");
                if !data.is_instance_of::<js_sys::ArrayBuffer>() { return Err("File bytes are required.".into()); }
                let bytes = js_sys::Uint8Array::new(&data);
                if bytes.length() == 0 || bytes.length() as usize > MAX_BYTES { return Err("Files must be between 1 byte and 50 MiB.".into()); }
                let opened = fileio::open_bytes(&mut self.app.session, &name, &bytes.to_vec(), None).map_err(|e| e.to_string())?;
                self.app.sync_views();
                self.name = name;
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                self.opened = true;
                event("opened", json!({"readOnly": self.read_only, "warnings": opened.get("warnings")}));
            }
            Some("set-read-only") => {
                if !self.opened { return Err("Open a file first.".into()); }
                self.app.session.commit_interaction().map_err(|e| e.to_string())?;
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                self.app.ui.dialog = None;
            }
            Some("export") => {
                if !self.opened || self.read_only { return Err("Enable editing before saving.".into()); }
                let request = field(&message, "requestId").as_string().ok_or("A save request identifier is required.")?;
                let format = field(&message, "format").as_string().unwrap_or_else(|| extension(&self.name));
                let format = match format.as_str() { "vctemplate" => "template", "drawcraft" | "ait" | "af" | "afdesign" | "afpub" => "vectorcraft", other => other };
                if !matches!(format, "svg" | "svgz" | "ai" | "eps" | "dxf" | "emf" | "wmf" | "vectorcraft" | "template" | "pdf" | "png") {
                    return Err("This vector export format is unsupported.".into());
                }
                self.app.session.commit_interaction().map_err(|e| e.to_string())?;
                let state = self.app.session.active().ok_or("No file is open.")?;
                let (bytes, warnings) = fileio::encode_with_warnings(&state.doc, format, &json!({"images": "embed"})).map_err(|e| e.to_string())?;
                if bytes.is_empty() || bytes.len() > MAX_BYTES { return Err("The exported file exceeds 50 MiB or is empty.".into()); }
                let revision = state.revision;
                self.last_exported = Some(revision);
                emit("exported", &json!({"requestId": request, "revision": revision, "warnings": warnings}).to_string(), &bytes);
            }
            Some("saved") => {
                if let Some(revision) = field(&message, "revision").as_f64() {
                    if let Some(state) = self.app.session.active_mut() {
                        if self.last_exported == Some(state.revision) && state.revision as f64 == revision {
                            state.mark_saved();
                            self.last_exported = None;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn report_dirty(&mut self) {
        let dirty = self.app.session.active().is_some_and(|state| state.is_dirty());
        if dirty != self.reported_dirty {
            self.reported_dirty = dirty;
            event("dirty", json!({"dirty": dirty}));
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        let Some(state) = self.app.session.active() else { return };
        if self.preview_revision != state.revision {
            self.textures.clear();
            self.preview_revision = state.revision;
        }
        let mut visible = Vec::new();
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                for (index, board) in state.doc.artboards.iter().enumerate() {
                    let longest = board.rect.width().max(board.rect.height()).max(1.0);
                    let scale = (2048.0 / longest).min(2.0);
                    let width = (board.rect.width() as f32).max(1.0);
                    let height = (board.rect.height() as f32).max(1.0);
                    let fit = ((ui.available_width() - 32.0).max(1.0) / width).min(1.5);
                    ui.add_space(16.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(width * fit, height * fit), egui::Sense::hover());
                    if !ui.is_rect_visible(rect) { continue; }
                    visible.push(index);
                    let texture = self.textures.entry(index).or_insert_with(|| {
                        let image = self.renderer.render_region(&state.doc, board.rect, scale, true);
                        ui.ctx().load_texture(format!("artboard-{index}"), egui::ColorImage::from_rgba_premultiplied([image.width as usize, image.height as usize], &image.pixels), egui::TextureOptions::LINEAR)
                    });
                    ui.painter().image(texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
            });
        });
        self.textures.retain(|index, _| visible.contains(index));
    }
}

impl eframe::App for Host {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for _ in 0..8 {
            let message = take();
            if message.is_null() || message.is_undefined() { break; }
            let request_id = field(&message, "requestId").as_string();
            if let Err(error) = self.receive(message) { event("error", json!({"requestId": request_id, "error": error})); }
        }
        if self.opened && !self.read_only { self.app.logic(ctx); }
        self.report_dirty();
    }
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        raw.dropped_files.clear();
        if !self.read_only { self.app.raw_input_hook(raw); }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.opened { return; }
        if self.read_only { self.preview(ui); } else { self.app.ui(ui); }
        self.report_dirty();
    }
}
