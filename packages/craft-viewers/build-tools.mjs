import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { acquireBuildLock } from "./build-lock.mjs";

export function ensureWasmBindgen({
	root,
	version,
	env = process.env,
	run = (command, args, options = {}) =>
		execFileSync(command, args, {
			cwd: root,
			stdio: "inherit",
			env,
			...options,
		}),
}) {
	const matchesVersion = (executable) => {
		try {
			return (
				run(executable, ["--version"], {
					env,
					encoding: "utf8",
					stdio: "pipe",
				}).trim() === `wasm-bindgen ${version}`
			);
		} catch {
			return false;
		}
	};
	if (env.WASM_BINDGEN) {
		if (!matchesVersion(env.WASM_BINDGEN))
			throw new Error(`WASM_BINDGEN must point to wasm-bindgen ${version}.`);
		return env.WASM_BINDGEN;
	}

	const directory = join(root, "target/craft-viewers/tools", version);
	const release = acquireBuildLock({
		directory,
		label: `wasm-bindgen ${version}`,
	});
	try {
		const binary =
			process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
		const executable = join(directory, "bin", binary);
		// Reuse tools installed by older builds before compiling a shared copy.
		const candidates = [
			executable,
			join(root, "target/wordcraft/tools", version, "bin", binary),
			...readdirSync(join(root, "packages/craft-viewers"))
				.filter((name) => /^[a-z]+craft$/.test(name))
				.sort()
				.map((name) =>
					join(
						root,
						"target/craft-viewers",
						name,
						"tools",
						version,
						"bin",
						binary,
					),
				),
		];
		const installed = candidates.find(
			(candidate) => existsSync(candidate) && matchesVersion(candidate),
		);
		if (installed) return installed;
		run(
			"cargo",
			[
				"install",
				"wasm-bindgen-cli",
				"--version",
				version,
				"--locked",
				"--root",
				directory,
				"--target-dir",
				join(directory, "target"),
			],
			{ env },
		);
		if (!matchesVersion(executable))
			throw new Error(
				`Installing wasm-bindgen ${version} did not produce the expected executable.`,
			);
		return executable;
	} finally {
		release();
	}
}
