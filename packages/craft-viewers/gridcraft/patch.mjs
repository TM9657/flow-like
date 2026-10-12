import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepare({ directory, original, write, replaceOnce }) {
	const uiPath = "crates/ui-egui/src/lib.rs";
	let ui = original(uiPath);
	ui = replaceOnce(
		ui,
		"pub struct Services {",
		"pub struct Services {\n    pub host_command: Option<Box<dyn Fn(&str) -> Option<Result<Json, String>>>>,",
		"host services",
	);
	const run =
		"pub fn run(&mut self, id: &str, params: Json) -> Result<Json, String> {";
	ui = replaceOnce(
		ui,
		run,
		`${run}\n        if let Some(result) = self.services.host_command.as_ref().and_then(|filter| filter(id)) { return result; }`,
		"command dispatch",
	);
	ui = replaceOnce(
		ui,
		"        ribbon::title_bar(self, ui);",
		"        if self.services.host_command.is_none() { ribbon::title_bar(self, ui); }",
		"title bar",
	);
	const backstage = "pub fn open_backstage(&mut self, page: backstage::Page) {";
	ui = replaceOnce(
		ui,
		backstage,
		`${backstage}\n        if self.services.host_command.is_some() { return; }`,
		"file screen",
	);
	ui = ui.replaceAll('"GridCraft"', '"Flows Sheets"');
	write(uiPath, ui);
	const enginePath = "crates/engine/src/lib.rs";
	let engine = original(enginePath);
	engine = replaceOnce(
		engine,
		"pub struct Session {",
		"pub struct Session {\n    pub hosted: bool,",
		"hosted session",
	);
	const execute =
		"pub fn execute(&mut self, id: &str, params: Json) -> Result<Json> {";
	engine = replaceOnce(
		engine,
		execute,
		`${execute}\n        if self.hosted && (id.starts_with("file.") || matches!(id, "window.close" | "window.activate" | "view.newWindow")) { return Err(EngineError::Other("Use the document controls above the editor.".into())); }`,
		"hosted file commands",
	);
	write(enginePath, engine);
	const ribbonPath = "crates/ui-egui/src/ribbon.rs";
	write(
		ribbonPath,
		replaceOnce(
			original(ribbonPath),
			"                    file_menu(app, ui);",
			"                    if app.services.host_command.is_none() { file_menu(app, ui); }",
			"File tab",
		),
	);
	const gridPath = "crates/ui-egui/src/grid.rs";
	let grid = original(gridPath);
	grid = replaceOnce(
		grid,
		"pub fn show(app: &mut SheetApp, ui: &mut egui::Ui) {",
		"pub fn show(app: &mut SheetApp, ui: &mut egui::Ui) { show_impl(app, ui, false); }\npub fn show_read_only(app: &mut SheetApp, ui: &mut egui::Ui) { show_impl(app, ui, true); }\nfn show_impl(app: &mut SheetApp, ui: &mut egui::Ui, read_only: bool) {",
		"read-only grid entry",
	);
	grid = replaceOnce(
		grid,
		"        if zoom_delta != 1.0 {",
		"        if zoom_delta != 1.0 && !read_only {",
		"preview zoom",
	);
	grid = replaceOnce(
		grid,
		"    // Interaction.\n    let previous_popups",
		"    if read_only {\n        let track_x = if rtl { (available.left(), rect.left()) } else { (rect.right(), available.right()) };\n        vertical_scrollbar(app, ui, sh, &geo, track_x, max_y, &t);\n        return;\n    }\n    // Interaction.\n    let previous_popups",
		"read-only interaction boundary",
	);
	write(gridPath, grid);
	write("Cargo.lock", original("Cargo.lock"));
	write(
		"apps/gridcraft-web/src/web.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	write(
		"apps/gridcraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod web;\n#[cfg(target_arch = "wasm32")]\nfn main() { web::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
}
