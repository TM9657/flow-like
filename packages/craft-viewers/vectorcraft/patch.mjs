import { readFileSync } from "node:fs";
import { join } from "node:path";
import { prepareLiveMcp } from "../live-mcp.mjs";

export default function prepare({
	source,
	directory,
	original,
	write,
	replaceOnce,
}) {
	const uiPath = "crates/ui-egui/src/lib.rs";
	let ui = original(uiPath);
	ui = replaceOnce(
		ui,
		"pub struct Services {",
		"pub struct Services {\n    pub host_command: Option<Box<dyn FnMut(&str) -> ResultHostCommand>>,",
		"host services",
	);
	ui = replaceOnce(
		ui,
		"#[derive(Default)]\npub struct Services",
		"pub type ResultHostCommand = Option<Result<Value, String>>;\n\n#[derive(Default)]\npub struct Services",
		"host result",
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
		'            egui::Panel::top("flows-vector-menu").show(ui, |ui| { menus::menu_bar(self, ui); });',
		"embedded menu bar",
	);
	write(uiPath, ui);
	const menusPath = "crates/ui-egui/src/menus.rs";
	let menus = original(menusPath);
	menus = replaceOnce(
		menus,
		"    let tree = menu_tree_named(app.session.prefs.font_names_in_english);",
		'    let tree: Vec<_> = menu_tree_named(app.session.prefs.font_names_in_english).into_iter().filter(|(name, _)| !matches!(*name, "File" | "Help")).collect();',
		"embedded menus",
	);
	write(menusPath, menus);
	const unsavedPath = "crates/ui-egui/src/unsaved.rs";
	let unsaved = original(unsavedPath);
	const close =
		"pub fn close(app: &mut VectorcraftApp, i: usize) -> Result<Value, String> {";
	unsaved = replaceOnce(
		unsaved,
		close,
		`${close}\n    if let Some(result) = app.services.host_command.as_mut().and_then(|hook| hook("file.close")) { return result; }`,
		"tab close guard",
	);
	write(unsavedPath, unsaved);
	write(
		"crates/ui-egui/src/chrome.rs",
		original("crates/ui-egui/src/chrome.rs").replaceAll(
			"VectorCraft",
			"Flows Vector",
		),
	);
	const cargoPath = "apps/vectorcraft-web/Cargo.toml";
	write(
		cargoPath,
		replaceOnce(
			original(cargoPath),
			"[dependencies]\n",
			"[dependencies]\nserde_json = { workspace = true }\nvectorcraft-render = { workspace = true }\n",
			"host dependencies",
		),
	);
	let lock = original("Cargo.lock");
	const block = lock.match(
		/\[\[package\]\]\nname = "vectorcraft-web"\n[\s\S]*?(?=\n\[\[package\]\]|$)/,
	)?.[0];
	if (!block) throw new Error("The VectorCraft web lock entry is missing.");
	lock = replaceOnce(
		lock,
		block,
		replaceOnce(
			block,
			"dependencies = [\n",
			'dependencies = [\n "serde_json",\n "vectorcraft-render",\n',
			"locked host dependencies",
		),
		"web lock entry",
	);
	write("Cargo.lock", lock);
	write(
		"apps/vectorcraft-web/src/web.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	write(
		"apps/vectorcraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod web;\n#[cfg(target_arch = "wasm32")]\nfn main() { web::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
	prepareLiveMcp({ id: "vectorcraft", source, write, original, replaceOnce });
}
