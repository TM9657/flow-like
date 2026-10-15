import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
	cpSync,
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { wasmLinkerEnvironment } from "../device-crypto/build-web.mjs";

const crate = dirname(fileURLToPath(import.meta.url));
const root = resolve(crate, "../..");
const benchmark = join(root, "tests/board-render-browser/egui");
const target =
	process.env.BOARD_CANVAS_TARGET_DIR ||
	(existsSync(join(benchmark, "target"))
		? join(benchmark, "target")
		: join(root, "target/board-canvas"));
const tools = join(root, "target/board-canvas-tools");
const staging = join(target, "board-canvas-web");
const outputs = ["web", "desktop"].map((app) =>
	join(root, "apps", app, "public/board-canvas"),
);
const assets = ["Inter-Regular.ttf", "Inter-Medium.ttf", "OFL-Inter.txt"];
const artifacts = [
	"board_canvas.js",
	"board_canvas.d.ts",
	"board_canvas_bg.wasm",
	"board_canvas_bg.wasm.d.ts",
	...assets,
];
const offline = process.argv.includes("--offline") ? ["--offline"] : [];
const probe = (command, args) =>
	execFileSync(command, args, {
		cwd: root,
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	}).trim();
const rustc = process.env.RUSTC || "rustc";
const fingerprint = createHash("sha256");
function hashDirectory(path) {
	for (const entry of readdirSync(path, { withFileTypes: true }).sort((a, b) =>
		a.name.localeCompare(b.name),
	)) {
		const fullPath = join(path, entry.name);
		fingerprint.update(entry.name);
		if (entry.isDirectory()) hashDirectory(fullPath);
		else fingerprint.update(readFileSync(fullPath));
	}
}
hashDirectory(join(crate, "src"));
hashDirectory(join(crate, "assets"));
for (const name of ["Cargo.toml", "Cargo.lock", "build-web.mjs"]) {
	fingerprint.update(readFileSync(join(crate, name)));
}
fingerprint.update(probe(rustc, ["--version", "--verbose"]));
const sourceHash = fingerprint.digest("hex");
const stamp = join(staging, "source-hash");
if (
	process.argv.includes("--dev") &&
	existsSync(stamp) &&
	readFileSync(stamp, "utf8") === sourceHash &&
	outputs.every((output) =>
		artifacts.every((artifact) => existsSync(join(output, artifact))),
	)
) {
	console.log("Board canvas WASM and fonts are current.");
	process.exit(0);
}
try {
	const targetLib = probe(rustc, [
		"--print",
		"target-libdir",
		"--target",
		"wasm32-unknown-unknown",
	]);
	if (
		!readdirSync(targetLib).some(
			(name) => name.startsWith("libstd-") && name.endsWith(".rlib"),
		)
	) {
		throw new Error("WASM standard library is missing");
	}
} catch (cause) {
	throw new Error(
		"Board canvas requires the repository Rust toolchain and its WASM standard library. Run rustup target add wasm32-unknown-unknown from the repository root.",
		{ cause },
	);
}
const version = readFileSync(join(crate, "Cargo.lock"), "utf8").match(
	/\[\[package\]\]\s+name = "wasm-bindgen"\s+version = "([^"]+)"/,
)?.[1];
if (!version) throw new Error("Board canvas Cargo.lock must pin wasm-bindgen");
const binary =
	process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
const candidates = [
	process.env.WASM_BINDGEN,
	join(tools, "bin", binary),
	join(benchmark, "tools/bin", binary),
	join(root, "target/device-crypto-tools/bin", binary),
].filter(Boolean);
let executable = candidates.find((candidate) => {
	try {
		return probe(candidate, ["--version"]) === `wasm-bindgen ${version}`;
	} catch {
		return false;
	}
});
const env = {
	...process.env,
	RUSTC_WRAPPER: process.env.BOARD_CANVAS_RUSTC_WRAPPER ?? "",
	...wasmLinkerEnvironment({ directory: tools }),
	CARGO_TARGET_DIR: target,
};
const run = (command, args) =>
	execFileSync(command, args, { cwd: root, env, stdio: "inherit" });
if (!executable) {
	run("cargo", [
		"install",
		"wasm-bindgen-cli",
		"--version",
		version,
		"--locked",
		"--root",
		tools,
		"-j",
		"4",
		...offline,
	]);
	executable = join(tools, "bin", binary);
}
run("cargo", [
	"build",
	"--manifest-path",
	join(crate, "Cargo.toml"),
	"--target",
	"wasm32-unknown-unknown",
	"--release",
	"--locked",
	"-j",
	"4",
	...offline,
]);
mkdirSync(staging, { recursive: true });
run(executable, [
	join(target, "wasm32-unknown-unknown/release/board_canvas.wasm"),
	"--target",
	"web",
	"--out-dir",
	staging,
	"--out-name",
	"board_canvas",
]);
for (const asset of assets) {
	cpSync(join(crate, "assets", asset), join(staging, asset));
}
for (const output of outputs) {
	mkdirSync(output, { recursive: true });
	for (const artifact of artifacts) {
		cpSync(join(staging, artifact), join(output, artifact));
	}
}
writeFileSync(stamp, sourceHash);
const wasm = readFileSync(join(staging, "board_canvas_bg.wasm"));
console.log(
	JSON.stringify({
		outputs,
		wasmBytes: wasm.length,
		wasmGzipBytes: gzipSync(wasm).length,
	}),
);
