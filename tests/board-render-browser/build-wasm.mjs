import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { wasmLinkerEnvironment } from "../../packages/device-crypto/build-web.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const crate = join(here, "egui");
const tools = join(crate, "tools");
const target = join(crate, "target");
const output = join(crate, "pkg");
const lock = readFileSync(join(crate, "Cargo.lock"), "utf8");
const version = lock.match(
	/\[\[package\]\]\s+name = "wasm-bindgen"\s+version = "([^"]+)"/,
)?.[1];
if (!version) throw new Error("egui/Cargo.lock does not pin wasm-bindgen");
const binary =
	process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
const candidates = [
	process.env.WASM_BINDGEN,
	join(tools, "bin", binary),
	resolve(here, "../../target/device-crypto-tools/bin", binary),
].filter(Boolean);
try {
	const targetLib = execFileSync(
		process.env.RUSTC || "rustc",
		["--print", "target-libdir", "--target", "wasm32-unknown-unknown"],
		{ cwd: here, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
	).trim();
	if (
		!readdirSync(targetLib).some(
			(name) => name.startsWith("libstd-") && name.endsWith(".rlib"),
		)
	)
		throw new Error("WASM standard library is missing");
} catch (cause) {
	throw new Error(
		"The board benchmark requires the repository's Rust toolchain and wasm32-unknown-unknown standard library. From the repository root, run: rustup target add wasm32-unknown-unknown",
		{ cause },
	);
}
const env = {
	...process.env,
	RUSTC_WRAPPER: process.env.BOARD_BENCH_RUSTC_WRAPPER ?? "",
	...wasmLinkerEnvironment({ directory: tools }),
	CARGO_TARGET_DIR: target,
};
const run = (command, args) =>
	execFileSync(command, args, { cwd: here, env, stdio: "inherit" });
let executable = candidates.find((candidate) => {
	try {
		return (
			execFileSync(candidate, ["--version"], {
				encoding: "utf8",
				stdio: ["ignore", "pipe", "ignore"],
			}).trim() === `wasm-bindgen ${version}`
		);
	} catch {
		return false;
	}
});
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
		...(process.argv.includes("--offline") ? ["--offline"] : []),
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
	...(process.argv.includes("--offline") ? ["--offline"] : []),
]);
mkdirSync(output, { recursive: true });
run(executable, [
	join(
		target,
		"wasm32-unknown-unknown/release/flow_like_board_render_bench.wasm",
	),
	"--target",
	"web",
	"--out-dir",
	output,
	"--out-name",
	"board_bench",
]);
const artifact = readFileSync(join(output, "board_bench_bg.wasm"));
console.log(
	JSON.stringify({
		wasmBytes: artifact.length,
		wasmGzipBytes: gzipSync(artifact).length,
	}),
);
