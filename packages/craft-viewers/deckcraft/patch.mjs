import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepare({ directory, original, write, replaceOnce }) {
	const uiPath = "crates/ui-egui/src/lib.rs";
	let ui = original(uiPath);
	ui = replaceOnce(
		ui,
		"pub struct Services {",
		"pub struct Services {\n    pub host_command: Option<Box<dyn Fn(&str) -> Option<Result<Value, String>>>>,",
		"host services",
	);
	const run =
		"pub fn run(&mut self, id: &str, params: Value) -> Result<Value, String> {";
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
	for (const [signature, command] of [
		["pub fn save(&mut self) {", "file.save"],
		["pub fn save_as_dialog(&mut self, format: &str) {", "file.saveAs"],
	]) {
		ui = replaceOnce(
			ui,
			signature,
			`${signature}\n        if let Some(result) = self.services.host_command.as_ref().and_then(|filter| filter("${command}")) {\n            if let Err(error) = result { self.set_status(error); }\n            return;\n        }`,
			command,
		);
	}
	ui = replaceOnce(
		ui,
		'"About DeckCraft"',
		'"About Flows Slides"',
		"About command",
	);
	write(uiPath, ui);
	const enginePath = "crates/engine/src/lib.rs";
	let engine = original(enginePath);
	engine = replaceOnce(
		engine,
		"pub struct Session {",
		"pub struct Session {\n    pub hosted: bool,",
		"hosted session",
	);
	engine = replaceOnce(
		engine,
		"        Session {\n            docs:",
		"        Session {\n            hosted: false,\n            docs:",
		"session initialization",
	);
	const execute =
		"pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {";
	engine = replaceOnce(
		engine,
		execute,
		`${execute}\n        if self.hosted && id.starts_with("file.") { return Err(EngineError::Other("Use the document controls above the editor.".into())); }`,
		"hosted file commands",
	);
	write(enginePath, engine);
	const dialogsPath = "crates/ui-egui/src/dialogs.rs";
	let dialogs = original(dialogsPath);
	for (const [search, replacement] of [
		['"about" => "About DeckCraft",', '"about" => "About Flows Slides",'],
		[
			'egui::RichText::new("DeckCraft").font(theme::bold(22.0))',
			'egui::RichText::new("Flows Slides").font(theme::bold(22.0))',
		],
		[
			'ui.label("Presentations and slide shows, rebuilt from scratch in pure Rust.");',
			'ui.label("Presentation viewing and editing in Flow-Like, powered by DeckCraft.");',
		],
		[
			"        let (r, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());\n        paint_logo(ui.painter(), r);",
			"        ui.add_space(8.0);",
		],
	])
		dialogs = replaceOnce(dialogs, search, replacement, "product identity");
	write(dialogsPath, dialogs);
	const manifest = "apps/deckcraft-web/Cargo.toml";
	write(
		manifest,
		replaceOnce(
			original(manifest),
			"deckcraft-engine = { workspace = true }",
			"deckcraft-engine = { workspace = true }\ndeckcraft-render = { workspace = true }",
			"renderer dependency",
		),
	);
	const lock = original("Cargo.lock");
	const packageBlock = lock.match(
		/\[\[package\]\]\nname = "deckcraft-web"\n[\s\S]*?(?=\n\[\[package\]\]|$)/,
	)?.[0];
	if (!packageBlock)
		throw new Error("The DeckCraft web package changed in Cargo.lock.");
	write(
		"Cargo.lock",
		replaceOnce(
			lock,
			packageBlock,
			replaceOnce(
				packageBlock,
				' "deckcraft-engine",',
				' "deckcraft-engine",\n "deckcraft-render",',
				"locked renderer dependency",
			),
			"web package",
		),
	);
	write(
		"apps/deckcraft-web/src/web.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	write(
		"apps/deckcraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod web;\n#[cfg(target_arch = "wasm32")]\nfn main() { web::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
}
