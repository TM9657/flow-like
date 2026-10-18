import { cpSync, mkdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const entryPath = createRequire(import.meta.url).resolve(
	"@excalidraw/excalidraw",
);
const packageDirectory = resolve(dirname(entryPath), "../..");
const { version } = JSON.parse(
	readFileSync(join(packageDirectory, "package.json"), "utf8"),
);

export function prepareExcalidrawAssets(publicDirectory, basePath = "") {
	const destination = join(
		publicDirectory instanceof URL
			? fileURLToPath(publicDirectory)
			: publicDirectory,
		"excalidraw",
		version,
	);
	mkdirSync(destination, { recursive: true });
	// Excalidraw resolves font URLs beneath EXCALIDRAW_ASSET_PATH at runtime.
	cpSync(
		join(packageDirectory, "dist", "prod", "fonts"),
		join(destination, "fonts"),
		{ recursive: true },
	);
	return `${basePath}/excalidraw/${version}/`;
}
