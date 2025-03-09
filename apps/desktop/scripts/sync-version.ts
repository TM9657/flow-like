import fs from "fs";

const version = JSON.parse(fs.readFileSync("package.json", "utf-8")).version;
const cargoToml = fs.readFileSync("./src-tauri/Cargo.toml", "utf-8");
const newCargoToml = cargoToml.replace(
	/version = ".*"/,
	`version = "${version}"`,
);
fs.writeFileSync("./src-tauri/Cargo.toml", newCargoToml);
