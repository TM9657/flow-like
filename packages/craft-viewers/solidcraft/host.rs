#[path = "live_mcp.rs"]
mod live_mcp;
use js_sys::{ArrayBuffer, Reflect, Uint8Array};
use serde_json::{Value, json};
use solidcraft_engine::{Session, io};
use solidcraft_ui_egui::{Services, SolveApp};
use std::cell::{Cell, RefCell};
use wasm_bindgen::prelude::*;

const MAX_BYTES: usize = 100 * 1024 * 1024;
const EXTENSIONS: &[&str] = &[
    "solidcraft",
    "step",
    "stp",
    "iges",
    "igs",
    "stl",
    "obj",
    "3mf",
    "glb",
    "gltf",
];

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftEmit)]
    fn emit(kind: &str, detail: &str, bytes: &[u8]);
    #[wasm_bindgen(js_namespace = window, js_name = flowLikeCraftTake)]
    fn take() -> JsValue;
}

thread_local! {
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    static READ_ONLY: Cell<bool> = const { Cell::new(true) };
}

#[wasm_bindgen]
pub fn request_repaint() {
    CONTEXT.with(|ctx| {
        if let Some(ctx) = ctx.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

fn event(kind: &str, detail: Value) {
    emit(kind, &detail.to_string(), &[]);
}
fn field(value: &JsValue, name: &str) -> JsValue {
    Reflect::get(value, &name.into()).unwrap_or(JsValue::UNDEFINED)
}

fn host_command(id: &str) -> Option<Result<Value, String>> {
    let id = solidcraft_engine::find_command(id).map_or(id, |spec| spec.id);
    if READ_ONLY.get() && !live_mcp::policy::command_read_only(id) {
        return Some(Err("Enable editing before changing this model.".into()));
    }
    if matches!(id, "file.save" | "file.save_as") {
        event("save-request", json!({}));
        return Some(Ok(json!({"pending": "hostSave"})));
    }
    if id.starts_with("file.")
        || id.starts_with("doc.recover")
        || matches!(id, "doc.open" | "host.close")
        || id.contains("export")
        || id.starts_with("script.")
    {
        return Some(Err(
            "Use the file controls above the editor. Models save as .solidcraft files.".into(),
        ));
    }
    None
}

// A missing glTF companion can otherwise produce a successful partial import.
fn check_gltf(bytes: &[u8], extension: &str) -> Result<(), String> {
    let json_bytes = if extension == "gltf" {
        bytes
    } else if extension == "glb" {
        if bytes.len() < 20 || &bytes[..4] != b"glTF" {
            return Err("Invalid GLB header.".into());
        }
        let word = |offset: usize| {
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap_or_default()) as usize
        };
        if word(4) != 2 || word(8) != bytes.len() || &bytes[16..20] != b"JSON" {
            return Err("Invalid GLB container.".into());
        }
        bytes
            .get(20..20_usize.saturating_add(word(12)))
            .ok_or("Invalid GLB JSON chunk.")?
    } else {
        return Ok(());
    };
    let model: Value =
        serde_json::from_slice(json_bytes).map_err(|e| format!("Invalid glTF: {e}"))?;
    if model
        .get("buffers")
        .and_then(Value::as_array)
        .is_some_and(|buffers| {
            buffers.iter().any(|buffer| {
                buffer
                    .get("uri")
                    .and_then(Value::as_str)
                    .is_some_and(|uri| !uri.starts_with("data:"))
            })
        })
    {
        return Err("This model references a separate buffer file. Export a GLB with embedded buffers or a self-contained glTF and upload it again.".into());
    }
    Ok(())
}

pub fn start() {
    solidcraft_engine::set_host_command(Some(host_command));
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
        .ok_or("The model canvas is unavailable.")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Invalid model canvas.")?;
    let runner = eframe::WebRunner::new();
    let mut options = eframe::WebOptions {
        depth_buffer: 32,
        ..Default::default()
    };
    if web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|q| q.contains("webgl"))
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(create) =
            &mut options.wgpu_options.wgpu_setup
    {
        create.instance_descriptor.backends = eframe::wgpu::Backends::GL;
    }
    runner
        .start(
            canvas,
            options,
            Box::new(|cc| {
                CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(cc.egui_ctx.clone()));
                solidcraft_ui_egui::fonts::setup_fonts(&cc.egui_ctx);
                let mut app = SolveApp::new(Session::default(), Services::default());
                if let Some(rs) = &cc.wgpu_render_state {
                    app.set_wgpu(rs, 32, 0, 1);
                }
                app.home.open = false;
                Ok(Box::new(Host {
                    app,
                    opened: false,
                    read_only: true,
                    reported_dirty: false,
                    last_exported: None,
                }))
            }),
        )
        .await
        .map_err(|e| {
            format!("The model editor could not start. WebGPU or WebGL2 is required: {e:?}")
        })?;
    std::mem::forget(runner);
    Ok(())
}

struct Host {
    app: SolveApp,
    opened: bool,
    read_only: bool,
    reported_dirty: bool,
    last_exported: Option<u64>,
}

impl Host {
    fn receive(&mut self, message: JsValue) -> Result<(), String> {
        match field(&message, "type").as_string().as_deref() {
            Some("mcp-request") => {
                let ctx = CONTEXT
                    .with(|context| context.borrow().clone())
                    .ok_or("The editor is not ready.")?;
                let read_only = self.read_only || !self.opened;
                if let Some(response) =
                    live_mcp::receive(&message, read_only, &mut |method, params| {
                        let (request, _reply) =
                            solidcraft_ui_egui::ControlRequest::new(method, params);
                        match solidcraft_ui_egui::control::handle(&mut self.app, &ctx, &request) {
                            solidcraft_ui_egui::control::Outcome::Done(reply) => {
                                live_mcp::unwrap_control(reply)
                            }
                            _ => Err("This operation requires a later editor frame.".into()),
                        }
                    })
                {
                    event("mcp-response", response);
                }
                ctx.request_repaint();
            }

            Some("open") => {
                if self.opened {
                    return Err("A model is already open.".into());
                }
                let name = field(&message, "name")
                    .as_string()
                    .ok_or("A model name is required.")?;
                let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                if !EXTENSIONS.contains(&extension.as_str()) {
                    return Err("This model format is not supported.".into());
                }
                let data = field(&message, "bytes");
                if !data.is_instance_of::<ArrayBuffer>() {
                    return Err("Model bytes are required.".into());
                }
                let data = Uint8Array::new(&data);
                if data.length() == 0 || data.length() as usize > MAX_BYTES {
                    return Err("Models must be between 1 byte and 100 MiB.".into());
                }
                let bytes = data.to_vec();
                check_gltf(&bytes, &extension)?;
                let basename = name.rsplit(['/', '\\']).next().unwrap_or("model");
                let path = format!("/host/{basename}");
                io::vfs::insert(&path, bytes);
                solidcraft_engine::set_host_command(None);
                let result = self.app.session.execute("doc.open", &json!({"path": path}));
                solidcraft_engine::set_host_command(Some(host_command));
                let _ = io::vfs::remove(&path);
                let result = result.map_err(|e| e.to_string())?;
                let mut warnings: Vec<String> = result
                    .get("warnings")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                if result.get("errors").and_then(Value::as_u64).unwrap_or(0) > 0 {
                    warnings.push("Some features could not rebuild. Review the model's timeline before saving.".into());
                }
                self.app.session.mark_saved();
                self.app.home.open = false;
                self.app.fit_view();
                self.read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                READ_ONLY.set(self.read_only);
                self.opened = true;
                event(
                    "opened",
                    json!({"revision": self.app.session.revision, "readOnly": self.read_only, "warnings": warnings}),
                );
            }
            Some("set-read-only") => {
                let read_only = field(&message, "readOnly").as_bool().unwrap_or(true);
                if read_only && !self.read_only {
                    solidcraft_ui_egui::dialogs::cancel(&mut self.app);
                    if self.app.session.active_sketch.is_some() {
                        self.app.finish_sketch();
                    }
                    self.app.preview.reset();
                    solidcraft_ui_egui::dim_view::cancel();
                    self.app.tool = None;
                    self.app.params = None;
                    self.app.design_properties = None;
                }
                self.read_only = read_only;
                READ_ONLY.set(read_only);
            }
            Some("export") => {
                if !self.opened || self.read_only {
                    return Err("Enable editing before saving.".into());
                }
                if !self.app.session.pending_moves.is_empty() {
                    return Err(
                        "Capture or revert component positions in the editor before saving.".into(),
                    );
                }
                if self.app.dialog.is_some() || self.app.applying.is_some() {
                    return Err(
                        "Finish or cancel the current operation in the editor before saving."
                            .into(),
                    );
                }
                let request = field(&message, "requestId")
                    .as_string()
                    .ok_or("A save request identifier is required.")?;
                if field(&message, "format").as_string().as_deref() != Some("solidcraft") {
                    return Err(
                        "Models save as .solidcraft files to preserve their editing history."
                            .into(),
                    );
                }
                let bytes = self
                    .app
                    .session
                    .design_container(io::DesignCompression::Fast, false, false)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > MAX_BYTES {
                    return Err("The exported model exceeds 100 MiB.".into());
                }
                let revision = self.app.session.revision;
                self.last_exported = Some(revision);
                emit(
                    "exported",
                    &json!({"requestId": request, "revision": revision}).to_string(),
                    &bytes,
                );
            }
            Some("saved") => {
                if let Some(revision) = field(&message, "revision").as_f64()
                    && self.last_exported == Some(revision as u64)
                    && self.app.session.revision as f64 == revision
                {
                    self.app.session.mark_saved();
                    self.last_exported = None;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Fit model").clicked() {
                self.app.fit_view();
            }
            for (label, view) in [
                ("Isometric", "iso"),
                ("Front", "front"),
                ("Top", "top"),
                ("Right", "right"),
            ] {
                if ui.button(label).clicked() {
                    self.app.animate_view(view);
                }
            }
            ui.label("Drag to orbit · Shift-drag to pan · Scroll to zoom");
        });
        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(
            rect,
            ui.id().with("model_preview_navigation"),
            egui::Sense::drag(),
        );
        if response.dragged() {
            let (delta, pan) = ui.input(|i| (i.pointer.delta(), i.modifiers.shift));
            self.app.cancel_view_animation();
            if pan {
                self.app
                    .cam
                    .pan(delta.x as f64, delta.y as f64, rect.height() as f64);
            } else {
                self.app.cam.orbit(delta.x as f64, delta.y as f64);
            }
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.app.cancel_view_animation();
                self.app.cam.zoom_at((-scroll as f64 * 0.005).exp(), None);
            }
        }
        // The upstream viewport also reads raw context input, even in disabled UI.
        // Only the camera controls above receive it in preview mode.
        let preview_input = ui.ctx().input(Clone::clone);
        ui.ctx().input_mut(|input| {
            input.events.clear();
            input.pointer = Default::default();
            input.smooth_scroll_delta = egui::Vec2::ZERO;
        });
        ui.add_enabled_ui(false, |ui| {
            solidcraft_ui_egui::viewport::show(&mut self.app, ui)
        });
        ui.ctx().input_mut(|input| *input = preview_input);
    }
}

impl eframe::App for Host {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        raw.dropped_files.clear();
        if self.read_only {
            self.app.host_cancel_synthetic();
        }
        raw.hovered_files.clear();
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
        } else {
            self.app.raw_input_hook(raw);
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
        self.app.home.open = false;
        self.app.logic(ctx);
        let dirty = self.app.session.is_dirty() || !self.app.session.pending_moves.is_empty();
        if dirty != self.reported_dirty {
            self.reported_dirty = dirty;
            event("dirty", json!({"dirty": dirty}));
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.read_only || !self.opened {
            self.preview(ui);
        } else {
            self.app.ui(ui);
        }
    }
}
