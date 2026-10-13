import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepare({ directory, original, write, replaceOnce }) {
	const uiPath = "crates/ui-egui/src/lib.rs";
	let ui = original(uiPath);
	ui = replaceOnce(
		ui,
		"pub struct Services {",
		"pub struct Services {\n    pub host_command: Option<Box<dyn FnMut(&str) -> Option<Result<Value, String>>>>,",
		"host services",
	);
	const run =
		"    pub fn run(&mut self, id: &str, params: Value) -> Result<Value, String> {";
	ui = replaceOnce(
		ui,
		run,
		`${run}\n        if let Some(result) = self.services.host_command.as_mut().and_then(|hook| hook(id)) { return result; }`,
		"command guard",
	);
	ui = replaceOnce(
		ui,
		"            chrome::app_bar(self, ui);",
		'            egui::Panel::top("flows-design-menu").show(ui, |ui| { ui.horizontal(|ui| menus::menu_bar(self, ui)); });',
		"embedded menu bar",
	);
	write(uiPath, ui);
	const menusPath = "crates/ui-egui/src/menus.rs";
	let menus = original(menusPath);
	menus = replaceOnce(
		menus,
		"    for (menu, entries) in menus {",
		'    for (menu, entries) in menus.into_iter().filter(|(menu, _)| !matches!(*menu, "File" | "Help")) {',
		"embedded menus",
	);
	const activate =
		"pub fn activate(app: &mut DesignApp, id: &str, params: &Value) {";
	menus = replaceOnce(
		menus,
		activate,
		`${activate}\n    if let Some(result) = app.services.host_command.as_mut().and_then(|hook| hook(id)) { if let Err(error) = result { app.status(error); } return; }`,
		"menu command guard",
	);
	const close =
		"pub fn close_document(app: &mut DesignApp, index: Option<usize>) {";
	menus = replaceOnce(
		menus,
		close,
		`${close}\n    if app.services.host_command.is_some() { app.status("Use the file controls above the editor."); return; }`,
		"tab close guard",
	);
	write(menusPath, menus);
	const chromePath = "crates/ui-egui/src/chrome.rs";
	let chrome = original(chromePath).replaceAll("DesignCraft", "Flows Design");
	chrome = replaceOnce(
		chrome,
		"            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());\n            crate::about::paint_mark(ui, r, crate::about::BRAND);\n            ui.add_space(8.0);\n",
		"",
		"home brand artwork",
	);
	write(chromePath, chrome);
	const aboutPath = "crates/ui-egui/src/about.rs";
	let about = original(aboutPath);
	const markPath = about.match(
		/^\/\/\/ The ArtCraft mark outline[^\n]*\nconst MARK_PATH: &str = "[^"\n]+";\n\n/m,
	)?.[0];
	const markPainter = about.match(
		/^\/\/\/ Paint the ArtCraft mark[^\n]*\npub fn paint_mark\([\s\S]*?(?=^\/\/\/ A speech-bubble glyph)/m,
	)?.[0];
	if (!markPath || !markPainter)
		throw new Error("The DesignCraft brand artwork changed; review its removal.");
	// Hidden dialogs are still compiled; remove both the geometry and its painter.
	about = replaceOnce(about, markPath, "", "brand artwork geometry");
	about = replaceOnce(about, markPainter, "", "brand artwork painter");
	about = replaceOnce(
		about,
		"/// ArtCraft brand blue (from `docs/brand/artcraft-mark.svg`).\npub const BRAND: Color32 = Color32::from_rgb(0x4e, 0x7b, 0xfb);\n",
		"",
		"brand color",
	);
	about = replaceOnce(
		about,
		"            let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());\n            paint_mark(ui, r, BRAND);\n            ui.add_space(6.0);\n",
		"",
		"about brand artwork",
	);
	write(aboutPath, about);
	const cargoPath = "apps/designcraft-web/Cargo.toml";
	write(
		cargoPath,
		replaceOnce(
			original(cargoPath),
			"[dependencies]\n",
			"[dependencies]\ndesigncraft-idml = { workspace = true }\ndesigncraft-render = { workspace = true }\n",
			"host dependencies",
		),
	);
	let lock = original("Cargo.lock");
	const block = lock.match(
		/\[\[package\]\]\nname = "designcraft-web"\n[\s\S]*?(?=\n\[\[package\]\]|$)/,
	)?.[0];
	if (!block) throw new Error("The DesignCraft web lock entry is missing.");
	lock = replaceOnce(
		lock,
		block,
		replaceOnce(
			block,
			"dependencies = [\n",
			'dependencies = [\n "designcraft-idml",\n "designcraft-render",\n',
			"locked host dependencies",
		),
		"web lock entry",
	);
	write("Cargo.lock", lock);
	write(
		"apps/designcraft-web/src/web.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	write(
		"apps/designcraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod web;\n#[cfg(target_arch = "wasm32")]\nfn main() { web::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
}
