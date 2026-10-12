import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepare({ directory, original, write, replaceOnce }) {
	write("apps/soundcraft-web/src/web.rs", readFileSync(join(directory, "host.rs"), "utf8"));
	let ui = original("crates/ui-egui/src/lib.rs");
	ui = replaceOnce(ui, "pub struct Services {", "pub struct Services {\n    pub host_command: Option<Box<dyn Fn(&str) -> Option<Result<Value, String>>>>,", "host services");
	ui = replaceOnce(ui, "pub fn run(&mut self, id: &str, params: Value) -> Result<Value, String> {", "pub fn run(&mut self, id: &str, params: Value) -> Result<Value, String> {\n        if let Some(result) = self.services.host_command.as_ref().and_then(|hook| hook(id)) { return result; }", "command interception");
	write("crates/ui-egui/src/lib.rs", ui);
	write("crates/ui-egui/src/menus.rs", original("crates/ui-egui/src/menus.rs").replaceAll('"About SoundCraft"', '"About Flows Audio"').replaceAll('"SoundCraft"', '"Flows Audio"'));
}
