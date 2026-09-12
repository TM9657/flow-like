/**
 * Downloads the pinned (or a requested) llama.cpp release build for all supported
 * platforms and places the binaries in the Tauri binaries directories with the
 * naming each platform config expects.
 *
 * Usage:
 *   bun run scripts/update-llama-server.ts                  # uses pinned build
 *   bun run scripts/update-llama-server.ts --latest         # follows the newest stable release
 *   bun run scripts/update-llama-server.ts --tag b10809     # fetches a specific build
 *   bun run scripts/update-llama-server.ts --platform mac-arm  # single platform
 *
 * Environment:
 *   GITHUB_TOKEN  — optional, avoids rate limits
 *
 * Upstream ships two kinds of tags: `vX.Y.Z` stable releases, which carry no
 * binaries and exist to name a build, and `bNNNN` nightlies, which carry the
 * archives. `--latest` resolves the stable release and then follows its
 * `nightly-tag.txt` asset to the build that belongs to it.
 */

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const PINNED_TAG = "b10809"; // stable v0.4.0
const GITHUB_API = "https://api.github.com";
const OWNER = "ggml-org";
const REPO = "llama.cpp";
const NIGHTLY_TAG_ASSET = "nightly-tag.txt";
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const BINARIES_DIR = path.resolve(SCRIPT_DIR, "../src-tauri/binaries");

// Tauri externalBin requires a target-triple suffix on macOS/Linux executables and
// macOS dylibs. Windows DLLs listed as resources don't need a suffix.
interface PlatformConfig {
	/** Key used in release asset name */
	assetName: string;
	/** Archive type */
	archiveType: "tar.gz" | "zip";
	/** Output directory relative to BINARIES_DIR */
	outDir: string;
	/** Tauri target triple for binary suffixing */
	tauriTriple: string;
	/** Files to extract and how to rename them (release name → local name with Tauri suffix) */
	files: FileMapping[];
}

interface FileMapping {
	/** Filename inside the release archive (under the llama-bXXXX/ prefix) */
	src: string;
	/** Destination filename. Use {TRIPLE} as placeholder for tauriTriple. */
	dst: string;
	/** If true, make executable (chmod +x) */
	executable?: boolean;
}

/**
 * macOS dylibs ship as a symlink chain (libfoo.dylib → libfoo.0.dylib →
 * libfoo.0.X.Y.dylib). Copying the unversioned name follows it to the real file;
 * fixMacOsDylibNames then rewrites the install names to match.
 */
function macDylib(name: string): FileMapping {
	return { src: `${name}.dylib`, dst: `${name}.dylib-{TRIPLE}` };
}

/** Core Linux libraries are resolved through their soname, so ship that name. */
function linuxSoname(name: string): FileMapping {
	return { src: `${name}.so.0`, dst: `${name}.so.0` };
}

/** Backend plugins and the server implementation carry no version in their soname. */
function linuxPlain(name: string): FileMapping {
	return { src: `${name}.so`, dst: `${name}.so` };
}

function winDll(name: string): FileMapping {
	return { src: `${name}.dll`, dst: `${name}.dll` };
}

function winDllSuffixed(name: string): FileMapping {
	return { src: `${name}.dll`, dst: `${name}.dll-{TRIPLE}` };
}

/** x86-64 ships one CPU backend per microarchitecture; ggml picks at runtime. */
const X64_CPU_VARIANTS = [
	"alderlake",
	"cannonlake",
	"cascadelake",
	"cooperlake",
	"haswell",
	"icelake",
	"ivybridge",
	"piledriver",
	"sandybridge",
	"sapphirerapids",
	"skylakex",
	"sse42",
	"x64",
	"zen4",
] as const;

const PLATFORMS: Record<string, PlatformConfig> = {
	"mac-arm": {
		assetName: "llama-{TAG}-bin-macos-arm64.tar.gz",
		archiveType: "tar.gz",
		outDir: "mac/arm",
		tauriTriple: "aarch64-apple-darwin",
		files: [
			{ src: "llama-server", dst: "llama-server-{TRIPLE}", executable: true },
			macDylib("libllama-server-impl"),
			macDylib("libllama-common"),
			macDylib("libllama"),
			macDylib("libggml"),
			macDylib("libggml-base"),
			macDylib("libggml-blas"),
			macDylib("libggml-cpu"),
			macDylib("libggml-metal"),
			macDylib("libggml-rpc"),
			macDylib("libmtmd"),
		],
	},
	"mac-intel": {
		assetName: "llama-{TAG}-bin-macos-x64.tar.gz",
		archiveType: "tar.gz",
		outDir: "mac/intel",
		tauriTriple: "x86_64-apple-darwin",
		files: [
			{ src: "llama-server", dst: "llama-server-{TRIPLE}", executable: true },
			macDylib("libllama-server-impl"),
			macDylib("libllama-common"),
			macDylib("libllama"),
			macDylib("libggml"),
			macDylib("libggml-base"),
			macDylib("libggml-blas"),
			macDylib("libggml-cpu"),
			macDylib("libggml-rpc"),
			macDylib("libmtmd"),
		],
	},
	"win-x64": {
		assetName: "llama-{TAG}-bin-win-vulkan-x64.zip",
		archiveType: "zip",
		outDir: "win/x64",
		tauriTriple: "x86_64-pc-windows-msvc",
		files: [
			{
				src: "llama-server.exe",
				dst: "llama-server-{TRIPLE}.exe",
				executable: true,
			},
			winDll("llama-server-impl"),
			winDll("llama-common"),
			winDll("llama"),
			winDll("mtmd"),
			winDll("ggml"),
			winDll("ggml-base"),
			winDll("ggml-rpc"),
			winDll("ggml-vulkan"),
			...X64_CPU_VARIANTS.map((variant) => winDll(`ggml-cpu-${variant}`)),
			winDll("libomp"),
			{ src: "LICENSE-LLVM-OpenMP", dst: "LICENSE-LLVM-OpenMP" },
		],
	},
	"win-arm": {
		assetName: "llama-{TAG}-bin-win-cpu-arm64.zip",
		archiveType: "zip",
		outDir: "win/arm",
		tauriTriple: "aarch64-pc-windows-msvc",
		files: [
			{
				src: "llama-server.exe",
				dst: "llama-server-{TRIPLE}.exe",
				executable: true,
			},
			winDllSuffixed("llama-server-impl"),
			winDllSuffixed("llama-common"),
			winDllSuffixed("llama"),
			winDllSuffixed("mtmd"),
			winDllSuffixed("ggml"),
			winDllSuffixed("ggml-base"),
			winDllSuffixed("ggml-cpu"),
			winDllSuffixed("ggml-rpc"),
			winDllSuffixed("libomp"),
			{ src: "LICENSE-LLVM-OpenMP", dst: "LICENSE-LLVM-OpenMP" },
		],
	},
	"linux-x64": {
		assetName: "llama-{TAG}-bin-ubuntu-vulkan-x64.tar.gz",
		archiveType: "tar.gz",
		outDir: "linux/x64",
		tauriTriple: "x86_64-unknown-linux-gnu",
		files: [
			{
				src: "llama-server",
				dst: "llama-server-{TRIPLE}",
				executable: true,
			},
			linuxPlain("libllama-server-impl"),
			linuxSoname("libllama-common"),
			linuxSoname("libllama"),
			linuxSoname("libmtmd"),
			linuxSoname("libggml"),
			linuxSoname("libggml-base"),
			linuxPlain("libggml-rpc"),
			linuxPlain("libggml-vulkan"),
			...X64_CPU_VARIANTS.map((variant) =>
				linuxPlain(`libggml-cpu-${variant}`),
			),
		],
	},
};

/**
 * Files this script owns in an output directory. Everything else there is
 * produced by a sibling script — prepare-mlx.ts writes the MLX sidecar next to
 * the llama binaries, prepare-windows-prereqs.ts writes the MSVC runtime — and
 * must survive an update.
 */
const OWNED_ARTIFACT_PATTERNS: readonly RegExp[] = [
	/^_download\./,
	/^(lib)?llama[-.]/,
	/^(lib)?ggml[-.]/,
	/^(lib)?mtmd[-.]/,
	/^libomp/,
	/^LICENSE-LLVM-OpenMP$/,
];

function isOwnedArtifact(fileName: string): boolean {
	return OWNED_ARTIFACT_PATTERNS.some((pattern) => pattern.test(fileName));
}

function getHeaders(): Record<string, string> {
	const headers: Record<string, string> = {
		Accept: "application/vnd.github.v3+json",
		"User-Agent": "flow-like-llama-updater",
	};
	if (process.env.GITHUB_TOKEN) {
		headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
	}
	return headers;
}

interface ReleaseResponse {
	tag_name: string;
	assets?: { name: string; browser_download_url: string }[];
}

async function resolveTag(requested: string | "latest"): Promise<string> {
	if (requested !== "latest") return requested;

	const url = `${GITHUB_API}/repos/${OWNER}/${REPO}/releases/latest`;
	const resp = await fetch(url, { headers: getHeaders() });
	if (!resp.ok)
		throw new Error(`Failed to fetch latest release: ${resp.status}`);
	const data = (await resp.json()) as ReleaseResponse;

	if (/^b\d+$/.test(data.tag_name)) return data.tag_name;

	const pointer = data.assets?.find(
		(asset) => asset.name === NIGHTLY_TAG_ASSET,
	);
	if (!pointer) {
		throw new Error(
			`Stable release ${data.tag_name} has no ${NIGHTLY_TAG_ASSET} asset; cannot resolve a build with binaries`,
		);
	}

	const nightly = await fetch(pointer.browser_download_url, {
		headers: getHeaders(),
		redirect: "follow",
	});
	if (!nightly.ok)
		throw new Error(
			`Failed to read ${NIGHTLY_TAG_ASSET} for ${data.tag_name}: ${nightly.status}`,
		);
	const tag = (await nightly.text()).trim();
	console.log(`Stable release ${data.tag_name} → build ${tag}`);
	return tag;
}

async function downloadArchive(url: string, dest: string): Promise<void> {
	console.log(`  Downloading ${url}`);
	const resp = await fetch(url, {
		headers: getHeaders(),
		redirect: "follow",
	});
	if (!resp.ok) throw new Error(`Download failed: ${resp.status} ${url}`);
	const buffer = await resp.arrayBuffer();
	fs.writeFileSync(dest, Buffer.from(buffer));
}

function extractFiles(
	archivePath: string,
	archiveType: "tar.gz" | "zip",
	fileMap: Map<string, string>,
	outDir: string,
): void {
	const tmpDir = fs.mkdtempSync(path.join(outDir, ".extract-"));

	try {
		if (archiveType === "tar.gz") {
			execSync(`tar -xzf "${archivePath}" -C "${tmpDir}"`, {
				stdio: "pipe",
			});
		} else {
			execSync(`unzip -o "${archivePath}" -d "${tmpDir}"`, {
				stdio: "pipe",
			});
		}

		// Find the extracted directory (usually llama-bXXXX/)
		const entries = fs.readdirSync(tmpDir);
		let extractRoot = tmpDir;
		for (const entry of entries) {
			const full = path.join(tmpDir, entry);
			if (fs.statSync(full).isDirectory()) {
				extractRoot = full;
				break;
			}
		}

		const missing: string[] = [];
		for (const [srcName, dstName] of fileMap) {
			const srcPath = path.join(extractRoot, srcName);
			const dstPath = path.join(outDir, dstName);

			if (!fs.existsSync(srcPath)) {
				missing.push(srcName);
				continue;
			}

			fs.copyFileSync(srcPath, dstPath);
			console.log(`  ✓ ${dstName}`);
		}

		if (missing.length > 0) {
			throw new Error(
				`Release archive is missing expected files: ${missing.join(", ")}. Upstream packaging changed — update the platform file list before bumping the tag.`,
			);
		}
	} finally {
		fs.rmSync(tmpDir, { recursive: true, force: true });
	}
}

/**
 * Strips the version from a Mach-O library file name:
 * libllama.0.4.0.dylib → libllama, libggml-base.dylib → libggml-base.
 */
function machOStem(fileName: string): string {
	return path.basename(fileName).replace(/(\.\d+)*\.dylib$/, "");
}

/**
 * Upstream links its dylibs through versioned install names (@rpath/libllama.0.dylib).
 * The bundle flattens them to unversioned names, so both the ids and every
 * reference to them have to be rewritten.
 */
function fixMacOsDylibNames(outDir: string, config: PlatformConfig): void {
	if (!config.tauriTriple.includes("apple-darwin")) return;
	if (process.platform !== "darwin") return;

	const triple = config.tauriTriple;
	const machOPaths = config.files
		.filter((f) => f.executable || f.src.endsWith(".dylib"))
		.map((f) => path.join(outDir, f.dst.replace("{TRIPLE}", triple)))
		.filter((filePath) => fs.existsSync(filePath));
	const bundledStems = new Set(
		config.files
			.filter((f) => f.src.endsWith(".dylib"))
			.map((f) => machOStem(f.src)),
	);

	for (const fm of config.files) {
		if (!fm.src.endsWith(".dylib")) continue;
		const dstPath = path.join(outDir, fm.dst.replace("{TRIPLE}", triple));
		if (!fs.existsSync(dstPath)) continue;

		const otoolOut = execSync(`otool -D "${dstPath}"`, {
			encoding: "utf-8",
		});
		const lines = otoolOut.trim().split("\n");
		if (lines.length < 2) continue;
		const currentId = lines[1].trim();
		const desiredId = `@rpath/${machOStem(fm.src)}.dylib`;

		if (currentId !== desiredId) {
			execSync(`install_name_tool -id "${desiredId}" "${dstPath}"`, {
				stdio: "pipe",
			});
			console.log(`  🔧 Fixed dylib id: ${currentId} → ${desiredId}`);
		}
	}

	for (const machOPath of machOPaths) {
		const linkedLibraries = execSync(`otool -L "${machOPath}"`, {
			encoding: "utf-8",
		})
			.split("\n")
			.slice(1)
			.map((line) => line.trim().split(" ")[0])
			.filter(Boolean);

		for (const linked of linkedLibraries) {
			const stem = machOStem(linked);
			if (!bundledStems.has(stem)) continue;
			const desired = `@rpath/${stem}.dylib`;
			if (linked === desired) continue;
			execSync(
				`install_name_tool -change "${linked}" "${desired}" "${machOPath}"`,
				{ stdio: "pipe" },
			);
			console.log(
				`  🔗 Fixed dependency in ${path.basename(machOPath)}: ${linked} → ${desired}`,
			);
		}
	}
}

function cleanOwnedArtifacts(dir: string): void {
	if (!fs.existsSync(dir)) {
		fs.mkdirSync(dir, { recursive: true });
		return;
	}
	for (const file of fs.readdirSync(dir)) {
		const full = path.join(dir, file);
		if (!fs.statSync(full).isFile()) continue;
		if (!isOwnedArtifact(file)) continue;
		fs.unlinkSync(full);
	}
}

async function updatePlatform(
	platformKey: string,
	config: PlatformConfig,
	tag: string,
): Promise<void> {
	const assetName = config.assetName.replace("{TAG}", tag);
	const downloadUrl = `https://github.com/${OWNER}/${REPO}/releases/download/${tag}/${assetName}`;
	const outDir = path.join(BINARIES_DIR, config.outDir);
	const archivePath = path.join(
		outDir,
		`_download.${config.archiveType === "tar.gz" ? "tar.gz" : "zip"}`,
	);

	console.log(`\n[${platformKey}] → ${assetName}`);

	// Build file mapping
	const fileMap = new Map<string, string>();
	for (const fm of config.files) {
		fileMap.set(fm.src, fm.dst.replace("{TRIPLE}", config.tauriTriple));
	}

	cleanOwnedArtifacts(outDir);
	await downloadArchive(downloadUrl, archivePath);
	extractFiles(archivePath, config.archiveType, fileMap, outDir);

	// Set executable bits
	for (const fm of config.files) {
		if (fm.executable) {
			const dst = path.join(
				outDir,
				fm.dst.replace("{TRIPLE}", config.tauriTriple),
			);
			if (fs.existsSync(dst)) {
				fs.chmodSync(dst, 0o755);
			}
		}
	}

	fixMacOsDylibNames(outDir, config);

	// Clean up archive
	if (fs.existsSync(archivePath)) {
		fs.unlinkSync(archivePath);
	}
}

async function main() {
	const args = process.argv.slice(2);
	let requestedTag: string | "latest" = PINNED_TAG;
	let platformFilter: string | null = null;

	for (let i = 0; i < args.length; i++) {
		if (args[i] === "--latest") {
			requestedTag = "latest";
		} else if (args[i] === "--tag" && args[i + 1]) {
			requestedTag = args[i + 1];
			i++;
		} else if (args[i] === "--platform" && args[i + 1]) {
			platformFilter = args[i + 1];
			i++;
		}
	}

	const tag = await resolveTag(requestedTag);
	console.log(`Updating llama.cpp binaries to ${tag}`);
	console.log(`Binaries directory: ${BINARIES_DIR}`);

	const platforms = platformFilter
		? { [platformFilter]: PLATFORMS[platformFilter] }
		: PLATFORMS;

	if (platformFilter && !PLATFORMS[platformFilter]) {
		console.error(
			`Unknown platform: ${platformFilter}. Available: ${Object.keys(PLATFORMS).join(", ")}`,
		);
		process.exit(1);
	}

	for (const [key, config] of Object.entries(platforms)) {
		await updatePlatform(key, config, tag);
	}

	console.log(`\nDone! Updated to ${tag}.`);
	console.log(
		"Remember to update PINNED_TAG in this script if you used --latest or --tag.",
	);
}

main().catch((err) => {
	console.error("Fatal:", err);
	process.exit(1);
});
