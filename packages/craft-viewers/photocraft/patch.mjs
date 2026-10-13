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
		"            panels::title_bar(self, ui);",
		'            egui::Panel::top("flows-photo-menu").show(ui, |ui| { menus::menu_bar(self, ui); });',
		"embedded menu bar",
	);
	write(uiPath, ui);
	const menusPath = "crates/ui-egui/src/menus.rs";
	let menus = original(menusPath);
	const invoke =
		"pub fn invoke(app: &mut PhotocraftApp, ctx: &egui::Context, id: &str, params: Value) -> Result<Value, String> {";
	menus = replaceOnce(
		menus,
		invoke,
		`${invoke}\n    if let Some(result) = app.services.host_command.as_mut().and_then(|hook| hook(id)) { return result; }`,
		"menu command guard",
	);
	menus = replaceOnce(
		menus,
		"            for top in TOP_MENUS {",
		'            for top in TOP_MENUS.into_iter().filter(|top| !matches!(*top, "File" | "Help")) {',
		"embedded menus",
	);
	write(menusPath, menus);
	write(
		"crates/ui-egui/src/panels.rs",
		replaceOnce(
			original("crates/ui-egui/src/panels.rs"),
			"                if t.pro {\n                    crate::chrome_ui::home_button(app, ui);\n                    widgets::vline(ui, 22.0);\n                }\n",
			"",
			"embedded options bar",
		).replaceAll("PhotoCraft", "Flows Photo"),
	);
	const cargoPath = "apps/photocraft-web/Cargo.toml";
	write(
		cargoPath,
		replaceOnce(
			original(cargoPath),
			"[dependencies]\n",
			"[dependencies]\nserde_json = { workspace = true }\nphotocraft-compose = { workspace = true }\n",
			"host dependencies",
		),
	);
	let lock = original("Cargo.lock");
	const block = lock.match(
		/\[\[package\]\]\nname = "photocraft-web"\n[\s\S]*?(?=\n\[\[package\]\]|$)/,
	)?.[0];
	if (!block) throw new Error("The PhotoCraft web lock entry is missing.");
	lock = replaceOnce(
		lock,
		block,
		replaceOnce(
			block,
			"dependencies = [\n",
			'dependencies = [\n "photocraft-compose",\n',
			"locked host dependencies",
		),
		"web lock entry",
	);
	write("Cargo.lock", lock);
	write(
		"apps/photocraft-web/src/web.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	write(
		"apps/photocraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod web;\n#[cfg(target_arch = "wasm32")]\nfn main() { web::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
}
