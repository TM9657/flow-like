import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
	cpSync,
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	rmSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { gzipSync } from "node:zlib";
import { wasmLinkerEnvironment } from "../device-crypto/build-web.mjs";
import { acquireBuildLock } from "./build-lock.mjs";
import {
	assertNoBrandArtwork,
	bundleLicenses,
	hashLicenseInputs,
} from "./licenses.mjs";

const suiteDirectory = dirname(fileURLToPath(import.meta.url));
const root = resolve(suiteDirectory, "../..");
const id = process.argv[2];
if (!id || !/^[a-z]+craft$/.test(id))
	throw new Error("Pass a Craft runtime name.");
const directory = join(suiteDirectory, id);
const toolchain =
	process.env.RUSTUP_TOOLCHAIN ||
	readFileSync(join(root, "rust-toolchain.toml"), "utf8").match(
		/^channel\s*=\s*"([^"]+)"/m,
	)?.[1];
if (!toolchain)
	throw new Error("The repository Rust toolchain is not configured.");
const buildEnvironment = { ...process.env, RUSTUP_TOOLCHAIN: toolchain };
let pin = JSON.parse(readFileSync(join(directory, "upstream.json"), "utf8"));
if (!/^[a-f0-9]{40}$/.test(pin.revision))
	throw new Error("Craft runtimes must be pinned to an exact Git commit.");
const cache = join(root, "target/craft-viewers", id);
const releaseLock = acquireBuildLock({ directory: cache, label: pin.label });
// An adapter may be updated while an earlier cold build owns this lock.
// Read its pin and fingerprint only after that build has finished.
pin = JSON.parse(readFileSync(join(directory, "upstream.json"), "utf8"));
if (!/^[a-f0-9]{40}$/.test(pin.revision))
	throw new Error("Craft runtimes must be pinned to an exact Git commit.");
const source =
	process.env[`CRAFT_SOURCE_${id.toUpperCase()}`] ||
	join(cache, "source", pin.revision);
const toolCandidates = [
	join(root, "target/wordcraft/tools", pin.wasmBindgen),
	...readdirSync(suiteDirectory).map((name) =>
		join(root, "target/craft-viewers", name, "tools", pin.wasmBindgen),
	),
];
const tools =
	toolCandidates.find((path) =>
		existsSync(
			join(
				path,
				"bin",
				process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen",
			),
		),
	) ?? join(cache, "tools", pin.wasmBindgen);
const output = join(cache, "runtime");
const hash = createHash("sha256").update(toolchain);
for (const base of [directory, suiteDirectory]) {
	for (const name of readdirSync(base).sort()) {
		const path = join(base, name);
		if (!statSync(path).isFile()) continue;
		hash.update(path.slice(root.length)).update(readFileSync(path));
	}
}
for (const name of [
	"packages/device-crypto/build-web.mjs",
	"rust-toolchain.toml",
])
	hash.update(name).update(readFileSync(join(root, name)));
hashLicenseInputs(hash);
const fingerprint = hash.digest("hex");
const stampPath = join(output, "runtime.json");
const targets = [
	`apps/web/public/craft/${id}`,
	`apps/desktop/public/craft/${id}`,
];

function run(command, args, options = {}) {
	return execFileSync(command, args, {
		cwd: source,
		stdio: "inherit",
		env: buildEnvironment,
		...options,
	});
}

function original(path) {
	return run("git", ["show", `${pin.revision}:${path}`], {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "inherit"],
	});
}

function replaceOnce(text, search, replacement, name) {
	if (text.split(search).length !== 2) {
		throw new Error(
			`${pin.label} ${pin.revision} changed ${name}; review the host adapter before upgrading.`,
		);
	}
	return text.replace(search, replacement);
}

function digest(path) {
	return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function cached() {
	if (!existsSync(stampPath)) return false;
	try {
		const stamp = JSON.parse(readFileSync(stampPath, "utf8"));
		return (
			stamp.fingerprint === fingerprint &&
			Object.entries(stamp.files).every(
				([name, sha]) =>
					existsSync(join(output, name)) && digest(join(output, name)) === sha,
			)
		);
	} catch {
		return false;
	}
}

function stage() {
	for (const target of targets) {
		rmSync(join(root, target), { recursive: true, force: true });
		mkdirSync(join(root, target), { recursive: true });
		cpSync(output, join(root, target), { recursive: true });
	}
	console.log(
		`${pin.label} ${pin.version} (${pin.revision.slice(0, 12)}) ready for web and desktop.`,
	);
}

if (!cached()) {
	mkdirSync(source, { recursive: true });
	if (!existsSync(join(source, ".git"))) {
		run("git", ["init", "--quiet"]);
	}
	let revision = "";
	try {
		revision = run("git", ["rev-parse", "HEAD"], {
			encoding: "utf8",
			stdio: "pipe",
		}).trim();
	} catch {}
	if (
		revision !== pin.revision &&
		process.env[`CRAFT_SOURCE_${id.toUpperCase()}`] &&
		revision
	) {
		throw new Error(
			`Craft source override must be checked out at ${pin.revision}.`,
		);
	}
	if (revision !== pin.revision) {
		run("git", ["fetch", "--depth", "1", pin.repository, pin.revision]);
		run("git", ["checkout", "--detach", pin.revision]);
	}
	const upstreamVersion = original("Cargo.toml").match(
		/\[workspace.package\]\s+version = "([^"]+)"/,
	)?.[1];
	if (upstreamVersion !== pin.version)
		throw new Error(`Set upstream.json version to ${upstreamVersion}.`);

	const lock = original("Cargo.lock");
	const lockedBindgen = lock.match(
		/\[\[package\]\]\nname = "wasm-bindgen"\nversion = "([^"]+)"/,
	)?.[1];
	if (lockedBindgen !== pin.wasmBindgen)
		throw new Error(`Set ${id}/upstream.json wasmBindgen to ${lockedBindgen}.`);
	const { default: prepare } = await import(
		pathToFileURL(join(directory, "patch.mjs")).href
	);
	const write = (path, text) => {
		mkdirSync(dirname(join(source, path)), { recursive: true });
		if (
			!existsSync(join(source, path)) ||
			readFileSync(join(source, path), "utf8") !== text
		)
			writeFileSync(join(source, path), text);
	};
	const prepared =
		(await prepare({
			source,
			directory,
			pin,
			original,
			write,
			replaceOnce,
			run,
			root,
		})) || {};

	const executable =
		process.env.WASM_BINDGEN ||
		join(
			tools,
			"bin",
			process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen",
		);
	let installed = "";
	try {
		installed = run(executable, ["--version"], {
			encoding: "utf8",
			stdio: "pipe",
		}).trim();
	} catch {}
	if (installed !== `wasm-bindgen ${pin.wasmBindgen}`) {
		if (process.env.WASM_BINDGEN)
			throw new Error(
				`WASM_BINDGEN must point to wasm-bindgen ${pin.wasmBindgen}.`,
			);
		run("cargo", [
			"install",
			"wasm-bindgen-cli",
			"--version",
			pin.wasmBindgen,
			"--locked",
			"--root",
			tools,
			"--target-dir",
			join(tools, "target"),
		]);
	}
	const cargoTarget =
		process.env[`CRAFT_TARGET_${id.toUpperCase()}`] ||
		join(root, "target/craft-viewers/cargo");
	const env = {
		...buildEnvironment,
		...wasmLinkerEnvironment({ directory: tools, env: buildEnvironment }),
		CARGO_TARGET_DIR: cargoTarget,
		CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || "4",
		// Release jobs pass native linker shims through these. They cannot be linked
		// into WebAssembly; the explicit --target below also overrides CARGO_BUILD_TARGET.
		RUSTFLAGS: undefined,
		CARGO_ENCODED_RUSTFLAGS: undefined,
	};
	// Optional system font checkouts would change the runtime without changing its
	// pin or cache key. This build uses only upstream's bundled, attributed fonts.
	env.CRAFT_FONTS_DIR = join(root, "target/craft-viewers/bundled-fonts-only");
	mkdirSync(env.CRAFT_FONTS_DIR, { recursive: true });
	env.CRAFT_FONTS_REQUIRED = undefined;
	const installedTargets = run("rustup", ["target", "list", "--installed"], {
		encoding: "utf8",
		stdio: "pipe",
	});
	if (!installedTargets.split(/\s+/).includes("wasm32-unknown-unknown"))
		run("rustup", ["target", "add", "wasm32-unknown-unknown"]);
	run(
		"cargo",
		[
			"build",
			"--locked",
			"--release",
			"-p",
			pin.cargoPackage,
			"--target",
			"wasm32-unknown-unknown",
			...(prepared.cargoArgs || []),
		],
		{ env },
	);
	rmSync(output, { recursive: true, force: true });
	mkdirSync(output, { recursive: true });
	run(executable, [
		join(
			cargoTarget,
			`wasm32-unknown-unknown/release/${pin.wasmFile || pin.cargoPackage}.wasm`,
		),
		"--target",
		"web",
		"--out-dir",
		output,
		"--out-name",
		"craft",
	]);
	// Pages limits each static asset to 25 MiB. Gzip also keeps the cold load small;
	// chunks leave room for upstream growth without changing hosting configuration.
	const wasmPath = join(output, "craft_bg.wasm");
	const wasmBytes = readFileSync(wasmPath);
	assertNoBrandArtwork(source, wasmBytes);
	const compressed = gzipSync(wasmBytes, { level: 9 });
	const wasm = {
		compression: "gzip",
		byteLength: wasmBytes.byteLength,
		sha256: digest(wasmPath),
		parts: [],
	};
	const chunkSize = 8 * 1024 * 1024;
	for (let offset = 0; offset < compressed.byteLength; offset += chunkSize) {
		const name = `craft.wasm.gz.${String(wasm.parts.length).padStart(3, "0")}`;
		const bytes = compressed.subarray(offset, offset + chunkSize);
		writeFileSync(join(output, name), bytes);
		wasm.parts.push({
			file: name,
			byteLength: bytes.byteLength,
			sha256: digest(join(output, name)),
		});
	}
	rmSync(wasmPath);
	for (const name of ["host.js", "index.html"])
		cpSync(join(suiteDirectory, name), join(output, name));
	cpSync(join(suiteDirectory, "protocol.mjs"), join(output, "protocol.js"));
	for (const asset of prepared.assets || []) {
		mkdirSync(dirname(join(output, asset.to)), { recursive: true });
		cpSync(join(source, asset.from), join(output, asset.to), {
			recursive: true,
		});
	}
	const licenses = join(output, "licenses");
	mkdirSync(licenses, { recursive: true });
	const attributionFiles = run(
		"git",
		["ls-tree", "-r", "--name-only", pin.revision],
		{ encoding: "utf8", stdio: "pipe" },
	)
		.trim()
		.split("\n")
		.filter((name) =>
			/(^|\/)(LICENSE[^/]*|NOTICE[^/]*|COPYING[^/]*|OFL[^/]*|ATTRIBUTION\.md|CGATS_LICENSE\.txt)$/i.test(
				name,
			),
		);
	for (const name of attributionFiles) {
		const destination = join(licenses, name.replace(/\.md$/, ".txt"));
		mkdirSync(dirname(destination), { recursive: true });
		writeFileSync(destination, original(name));
	}
	bundleLicenses({
		source,
		output,
		pin,
		cargoPackage: pin.cargoPackage,
		cargoArgs: prepared.cargoArgs || [],
		env,
	});
	const files = {};
	for (const name of readdirSync(output, { recursive: true })) {
		if (name === "runtime.json") continue;
		try {
			if (readFileSync(join(output, name)).byteLength > 25 * 1024 * 1024)
				throw new Error(
					`${pin.label} asset ${name} exceeds the static hosting limit of 25 MiB.`,
				);
			files[name] = digest(join(output, name));
		} catch (error) {
			if (error.code !== "EISDIR") throw error;
		}
	}
	writeFileSync(
		stampPath,
		`${JSON.stringify(
			{
				...pin,
				id,
				maxBytes: pin.maxBytes || 200 * 1024 * 1024,
				fingerprint,
				lockSha256: digest(join(source, "Cargo.lock")),
				wasm,
				files,
			},
			null,
			2,
		)}\n`,
	);
}
stage();
releaseLock();
process.removeListener("exit", releaseLock);
