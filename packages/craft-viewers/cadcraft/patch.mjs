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
	write(
		"apps/cadcraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod host;\n#[cfg(target_arch = "wasm32")]\nfn main() { host::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
	write(
		"apps/cadcraft-web/src/host.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	let engine = original("crates/engine/src/lib.rs");
	engine = replaceOnce(
		engine,
		"pub struct Session {",
		"pub struct Session {\n    pub host_command: Option<fn(&str) -> Option<std::result::Result<Value, String>>>,",
		"host commands",
	);
	engine = replaceOnce(
		engine,
		"            docs: Vec::new(),",
		"            host_command: None,\n            docs: Vec::new(),",
		"host command default",
	);
	engine = replaceOnce(
		engine,
		"        let spec = find_command(id).ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;",
		"        let spec = find_command(id).ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;\n        if let Some(result) = self.host_command.and_then(|hook| hook(spec.id)) { return result.map_err(EngineError::Other); }",
		"programmatic commands",
	);
	engine = replaceOnce(
		engine,
		"        let spec = find_command(&id).ok_or_else(|| EngineError::UnknownCommand(name.to_string()))?;",
		"        let spec = find_command(&id).ok_or_else(|| EngineError::UnknownCommand(name.to_string()))?;\n        if let Some(result) = self.host_command.and_then(|hook| hook(spec.id)) { return result.map(|_| ()).map_err(EngineError::Other); }",
		"interactive commands",
	);
	write("crates/engine/src/lib.rs", engine);
	let menus = original("crates/ui-egui/src/menus.rs");
	menus = replaceOnce(
		menus,
		"pub fn run_ui_command(app: &mut CadApp, id: &str, params: &Value) -> Option<Result<Value, String>> {",
		"pub fn run_ui_command(app: &mut CadApp, id: &str, params: &Value) -> Option<Result<Value, String>> {\n    if let Some(result) = app.session.host_command.and_then(|hook| hook(id)) { return Some(result); }",
		"UI commands",
	);
	write(
		"crates/ui-egui/src/menus.rs",
		menus.replaceAll("CADCraft", "Flows CAD"),
	);
	write(
		"crates/ui-egui/src/chrome.rs",
		original("crates/ui-egui/src/chrome.rs").replaceAll(
			"CADCraft",
			"Flows CAD",
		),
	);
	prepareLiveMcp({ id: "cadcraft", source, write, original, replaceOnce });
}
