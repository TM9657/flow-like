import fs from "fs";
import path from "path";

const BIN_DIR = "./src-tauri/bin";
const RELEASES_API =
	"https://api.github.com/repos/TM9657/docling-binary/releases/latest";

interface Binary {
	out: string;
	name: string;
}

const binaries: readonly Binary[] = [
	{
		out: "flow-docling-aarch64-apple-darwin",
		name: "flow-docling-macos-arm64",
	},
	{ out: "flow-docling-x86_64-apple-darwin", name: "flow-docling-macos-x64" },
	{
		out: "flow-docling-x86_64-unknown-linux-gnu",
		name: "flow-docling-linux-x64",
	},
	{
		out: "flow-docling-x86_64-pc-windows-msvc.exe",
		name: "flow-docling-windows-x64.exe",
	},
];

async function fetchJson<T>(url: string): Promise<T> {
	const headers: Record<string, string> = {
		"User-Agent": "flow-like-downloader",
		Accept: "application/vnd.github+json",
	};

	const githubToken = process.env.GITHUB_TOKEN;
	if (githubToken) {
		headers.Authorization = `Bearer ${githubToken}`;
	}

	const res = await fetch(url, { headers });

	if (!res.ok) {
		throw new Error(`Failed to fetch ${url}: ${res.status} ${res.statusText}`);
	}
	return res.json() as Promise<T>;
}

async function downloadFile(url: string, dest: string) {
	const headers: Record<string, string> = {};

	const githubToken = process.env.GITHUB_TOKEN;
	if (githubToken) {
		headers.Authorization = `Bearer ${githubToken}`;
	}

	await fs.promises.mkdir(path.dirname(dest), { recursive: true });
	const res = await fetch(url, {
		headers: headers,
	});
	if (!res.ok) {
		throw new Error(
			`Failed to download ${url}: ${res.status} ${res.statusText}`,
		);
	}
	const fileStream = fs.createWriteStream(dest, { mode: 0o755 });
	// @ts-ignore Bun's Response.body is a ReadableStream
	const reader = res.body?.getReader();
	if (!reader) throw new Error("No response body");

	try {
		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			fileStream.write(Buffer.from(value));
		}
		fileStream.end();
		await fs.promises.chmod(dest, 0o755);
	} catch (e) {
		fileStream.destroy();
		throw e;
	}
}

async function main() {
	await fs.promises.mkdir(BIN_DIR, { recursive: true });
	const release = await fetchJson<any>(RELEASES_API);

	for (const bin of binaries) {
		const asset = release.assets?.find((a: any) => a.name === bin.name);
		if (!asset) {
			console.warn(`Asset not found: ${bin.name}`);
			continue;
		}
		const outPath = path.join(BIN_DIR, bin.out);
		if (fs.existsSync(outPath)) {
			console.log(`Already exists: ${bin.out}`);
			continue;
		}
		console.log(`Downloading ${bin.name}...`);
		try {
			await downloadFile(asset.browser_download_url, outPath);
			console.log(`Downloaded to ${outPath}`);
		} catch (e) {
			console.error(`Failed to download ${bin.name}:`, e);
		}
	}
}

main().catch((err) => {
	console.error("Error in download-flow-docling.ts:", err);
	process.exit(1);
});
