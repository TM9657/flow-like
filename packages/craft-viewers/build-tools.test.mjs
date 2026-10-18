import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import {
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { ensureWasmBindgen } from "./build-tools.mjs";

const version = "0.2.129";
const binary =
	process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";

function fixture(t) {
	const root = mkdtempSync(join(tmpdir(), "flow-craft-tools-"));
	mkdirSync(join(root, "packages/craft-viewers/cadcraft"), { recursive: true });
	t.after(() => rmSync(root, { recursive: true, force: true }));
	return root;
}

function writeTool(executable, toolVersion = version) {
	mkdirSync(dirname(executable), { recursive: true });
	writeFileSync(executable, `wasm-bindgen ${toolVersion}\n`);
}

test("reuses compatible WordCraft and editor tools from earlier builds", (t) => {
	for (const legacy of ["target/wordcraft", "target/craft-viewers/cadcraft"]) {
		const root = fixture(t);
		const executable = join(root, legacy, "tools", version, "bin", binary);
		writeTool(executable);
		writeTool(
			join(root, "target/craft-viewers/tools", version, "bin", binary),
			"0.2.100",
		);
		assert.equal(
			ensureWasmBindgen({
				root,
				version,
				env: {},
				run: (command) => readFileSync(command, "utf8"),
			}),
			executable,
		);
		assert.equal(
			existsSync(
				join(root, "target/craft-viewers/tools", version, "build.lock"),
			),
			false,
		);
	}
});

test("installs a shared tool with the supplied Cargo job budget and verifies its version", (t) => {
	const root = fixture(t);
	const directory = join(root, "target/craft-viewers/tools", version);
	const executable = join(directory, "bin", binary);
	const env = { CARGO_BUILD_JOBS: "2" };
	let installs = 0;
	const run = (command, args, options) => {
		assert.equal(options.env, env);
		if (command !== "cargo") return readFileSync(command, "utf8");
		installs++;
		assert.deepEqual(args, [
			"install",
			"wasm-bindgen-cli",
			"--version",
			version,
			"--locked",
			"--root",
			directory,
			"--target-dir",
			join(directory, "target"),
		]);
		assert.equal(existsSync(join(directory, "build.lock")), true);
		writeTool(executable);
	};
	assert.equal(ensureWasmBindgen({ root, version, env, run }), executable);
	assert.equal(ensureWasmBindgen({ root, version, env, run }), executable);
	assert.equal(installs, 1);
	assert.equal(existsSync(join(directory, "build.lock")), false);
});

test("validates WASM_BINDGEN overrides without falling back to installation", (t) => {
	const root = fixture(t);
	const executable = join(root, binary);
	const env = { WASM_BINDGEN: executable };
	const run = (command) => readFileSync(command, "utf8");
	writeTool(executable);
	assert.equal(ensureWasmBindgen({ root, version, env, run }), executable);
	writeTool(executable, "0.2.100");
	assert.throws(
		() => ensureWasmBindgen({ root, version, env, run }),
		/WASM_BINDGEN must point to wasm-bindgen 0\.2\.129/,
	);
	assert.equal(existsSync(join(root, "target")), false);
});

test("releases the provisioning lock after installation fails", (t) => {
	const root = fixture(t);
	assert.throws(
		() =>
			ensureWasmBindgen({
				root,
				version,
				env: {},
				run: () => {
					throw new Error("install failed");
				},
			}),
		/install failed/,
	);
	assert.equal(
		existsSync(join(root, "target/craft-viewers/tools", version, "build.lock")),
		false,
	);
});

test("concurrent editor processes install the same tool only once", async (t) => {
	const root = fixture(t);
	const script = `
		import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
		import { join } from "node:path";
		import { ensureWasmBindgen } from ${JSON.stringify(new URL("./build-tools.mjs", import.meta.url).href)};
		const root = process.argv[1];
		ensureWasmBindgen({
			root, version: ${JSON.stringify(version)}, env: {},
			run(command, args) {
				if (command !== "cargo") return readFileSync(command, "utf8");
				appendFileSync(join(root, "installs.log"), "install\\n");
				Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 150);
				const directory = args[args.indexOf("--root") + 1];
				mkdirSync(join(directory, "bin"), { recursive: true });
				writeFileSync(join(directory, "bin", ${JSON.stringify(binary)}), "wasm-bindgen ${version}\\n");
			},
		});
	`;
	await Promise.all(
		Array.from(
			{ length: 3 },
			() =>
				new Promise((resolve, reject) => {
					const child = spawn(
						process.execPath,
						["--input-type=module", "-e", script, root],
						{ stdio: ["ignore", "ignore", "pipe"] },
					);
					let stderr = "";
					child.stderr.on("data", (data) => {
						stderr += data;
					});
					child.once("error", reject);
					child.once("exit", (code) =>
						code === 0
							? resolve()
							: reject(new Error(stderr || `Worker exited with ${code}`)),
					);
				}),
		),
	);
	assert.equal(readFileSync(join(root, "installs.log"), "utf8"), "install\n");
});
