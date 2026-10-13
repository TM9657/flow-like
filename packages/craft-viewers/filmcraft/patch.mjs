import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepare({ directory, original, write, replaceOnce }) {
	let lib = original("apps/filmcraft-web/src/lib.rs");
	lib = replaceOnce(
		lib,
		"pub mod api;",
		"mod host;\npub mod api;",
		"host module",
	);
	lib = replaceOnce(
		lib,
		"        self.autosave.tick(&self.app.session, ctx);",
		"",
		"browser autosave isolation",
	);
	write("apps/filmcraft-web/src/lib.rs", lib);
	write(
		"apps/filmcraft-web/src/host.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	let engine = original("crates/engine/src/lib.rs");
	engine = replaceOnce(
		engine,
		"pub struct Session {",
		"pub struct Session {\n    pub host_command: Option<fn(&str) -> Option<std::result::Result<Value, String>>>,",
		"session host command",
	);
	engine = replaceOnce(
		engine,
		'            project: Arc::new(Project::new("Untitled")),',
		'            host_command: None,\n            project: Arc::new(Project::new("Untitled")),',
		"session host default",
	);
	engine = replaceOnce(
		engine,
		"pub fn execute(&mut self, id: &str, params: Value) -> Result<Value> {",
		"pub fn execute(&mut self, id: &str, params: Value) -> Result<Value> {\n        if let Some(result) = self.host_command.and_then(|hook| hook(id)) { return result.map_err(EngineError::Other); }",
		"host save interception",
	);
	write("crates/engine/src/lib.rs", engine);
	let menus = original("crates/ui-egui/src/menus.rs");
	menus = replaceOnce(
		menus,
		"pub fn invoke(app: &mut FilmcraftApp, ctx: &egui::Context, id: &str, mut params: Value) -> Result<Value, String> {",
		"pub fn invoke(app: &mut FilmcraftApp, ctx: &egui::Context, id: &str, mut params: Value) -> Result<Value, String> {\n    if let Some(result) = app.session.host_command.and_then(|hook| hook(id)) { return result; }",
		"menu host interception",
	);
	menus = menus.replaceAll('"About FilmCraft"', '"About Flows Video"');
	write("crates/ui-egui/src/menus.rs", menus);
	write(
		"crates/ui-egui/src/brand.rs",
		`pub fn paint_wordmark(ui: &egui::Ui, left_center: egui::Pos2, height: f32, _dark: bool) -> Option<egui::Rect> {
    Some(ui.painter().text(left_center, egui::Align2::LEFT_CENTER, "Flows Video", egui::FontId::proportional(height), ui.visuals().text_color()))
}
`,
	);
	let header = original("crates/ui-egui/src/header.rs");
	header = replaceOnce(
		header,
		'let label = "Discord";',
		'let label = "Flows Video";',
		"header brand",
	);
	header = header
		.replaceAll(
			'"Join the ArtCraft Discord (discord.gg/artcraft)"',
			'"Flows Video"',
		)
		.replaceAll('"Join the ArtCraft Discord"', '"Flows Video"');
	header = replaceOnce(
		header,
		"            crate::links::open(app, ui.ctx(), crate::links::DISCORD);",
		'            app.ui.status = "Flows Video".into();',
		"header external link",
	);
	write("crates/ui-egui/src/header.rs", header);
	return {
		assets: [
			{
				from: "apps/filmcraft-web/web/audio-worklet.js",
				to: "audio-worklet.js",
			},
		],
	};
}
