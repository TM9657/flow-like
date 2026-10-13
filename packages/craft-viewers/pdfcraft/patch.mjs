import { readFileSync } from "node:fs";
import { join } from "node:path";

export default function prepare({ directory, original, write, replaceOnce }) {
	const uiPath = "crates/ui-egui/src/lib.rs";
	let ui = original(uiPath);
	ui = replaceOnce(
		ui,
		"        chrome::tab_strip(self, ui);",
		"        if self.host_save.is_none() { chrome::tab_strip(self, ui); }",
		"document tabs",
	);
	ui = replaceOnce(
		ui,
		'.map_or_else(|| "PdfCraft".to_owned(), |name| format!("{name} — PdfCraft"));',
		'.map_or_else(|| "Flows PDF".to_owned(), |name| format!("{name} - Flows PDF"));',
		"window title",
	);
	const open =
		"fn open_bytes_with_focus(&mut self, name: &str, path: Option<String>, bytes: Vec<u8>, activate: bool) -> Result<(), String> {";
	ui = replaceOnce(
		ui,
		open,
		`${open}\n        if self.host_save.is_some() && !self.views.is_empty() { return Err("Open another document from storage.".into()); }`,
		"single document",
	);
	for (const signature of [
		"pub fn open_dialog(&mut self) {",
		"pub fn close_tab(&mut self, index: usize) {",
	]) {
		ui = replaceOnce(
			ui,
			signature,
			`${signature}\n        if self.host_save.is_some() { return; }`,
			"host file controls",
		);
	}
	write(uiPath, ui);
	const commandsPath = "crates/ui-egui/src/commands.rs";
	const execute = "pub fn execute(&mut self, id: &str) -> bool {";
	write(
		commandsPath,
		replaceOnce(
			original(commandsPath),
			execute,
			`${execute}\n        if self.host_save.is_some() && ((id.starts_with("file.") && !matches!(id, "file.save" | "file.save_as" | "file.properties")) || id.starts_with("create.") || id == "page.combine") {\n            self.notify("Use the document controls above the editor.");\n            return true;\n        }`,
			"host commands",
		),
	);
	const editingPath = "crates/ui-egui/src/editing.rs";
	let editing = original(editingPath);
	const save =
		"pub(crate) fn save_then(&mut self, index: usize, target: SaveTarget, after: impl FnOnce(&mut Self) + Send + 'static) -> bool {";
	editing = replaceOnce(
		editing,
		save,
		`${save}\n        if let Some(save) = &self.host_save {\n            let _ = save("document.pdf", &[], matches!(target, SaveTarget::As));\n            return false;\n        }`,
		"asynchronous host save",
	);
	editing += `\n${readFileSync(join(directory, "host-methods.rs"), "utf8")}`;
	write(editingPath, editing);
	const chromePath = "crates/ui-egui/src/chrome.rs";
	let chrome = original(chromePath);
	chrome = replaceOnce(
		chrome,
		'        ui.menu_button(tl!("File"), |ui| crate::commands::registry_menu(app, ui, "File"));',
		'        if app.host_save.is_none() { ui.menu_button(tl!("File"), |ui| crate::commands::registry_menu(app, ui, "File")); }',
		"File menu",
	);
	write(chromePath, chrome);
	const dialogsPath = "crates/ui-egui/src/dialogs.rs";
	let dialogs = original(dialogsPath);
	for (const [search, replacement] of [
		["widgets::artcraft_mark(ui, 40.0);", "ui.add_space(8.0);"],
		[
			'egui::RichText::new("PdfCraft").font(theme::semibold(20.0))',
			'egui::RichText::new("Flows PDF").font(theme::semibold(20.0))',
		],
		[
			"widgets::artcraft_logo(ui, 16.0);",
			'ui.label("Powered by PdfCraft. © ArtCraft contributors.");',
		],
	])
		dialogs = replaceOnce(dialogs, search, replacement, "product identity");
	write(dialogsPath, dialogs);
	const homePath = "crates/ui-egui/src/home.rs";
	write(
		homePath,
		replaceOnce(
			original(homePath),
			"                        widgets::artcraft_mark(ui, 28.0);\n",
			"",
			"home brand artwork",
		),
	);
	const widgetsPath = "crates/ui-egui/src/widgets.rs";
	const widgets = original(widgetsPath);
	const brandWidgets = widgets.match(
		/^\/\/\/ The ArtCraft wordmark[^\n]*\n[\s\S]*?(?=^\/\/\/ Buttons for every community link)/m,
	)?.[0];
	if (!brandWidgets?.includes("pub fn artcraft_mark("))
		throw new Error("The PdfCraft brand widgets changed; review artwork removal.");
	// Remove the asset loaders too, so hidden UI cannot keep restricted logos in WASM.
	write(
		widgetsPath,
		replaceOnce(widgets, brandWidgets, "", "brand artwork loaders"),
	);
	const manifest = "apps/pdfcraft-web/Cargo.toml";
	write(
		manifest,
		replaceOnce(
			original(manifest),
			"\negui = { workspace = true }",
			'\negui = { workspace = true }\nserde_json = { workspace = true }\nwasm-bindgen = "0.2"',
			"bridge dependencies",
		),
	);
	const lock = original("Cargo.lock");
	const packageBlock = lock.match(
		/\[\[package\]\]\nname = "pdfcraft-web"\n[\s\S]*?(?=\n\[\[package\]\]|$)/,
	)?.[0];
	if (!packageBlock)
		throw new Error("The PdfCraft web package changed in Cargo.lock.");
	const dependencies = replaceOnce(
		packageBlock,
		' "pdfcraft-ui-egui",',
		' "pdfcraft-ui-egui",\n "serde_json",\n "wasm-bindgen",',
		"locked bridge dependencies",
	);
	write(
		"Cargo.lock",
		replaceOnce(lock, packageBlock, dependencies, "web package"),
	);
	write(
		"apps/pdfcraft-web/src/web.rs",
		readFileSync(join(directory, "host.rs"), "utf8"),
	);
	write(
		"apps/pdfcraft-web/src/main.rs",
		'#[cfg(target_arch = "wasm32")]\nmod web;\n#[cfg(target_arch = "wasm32")]\nfn main() { web::start(); }\n#[cfg(not(target_arch = "wasm32"))]\nfn main() {}\n',
	);
}
