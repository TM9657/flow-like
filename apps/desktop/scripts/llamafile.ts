import fs from "node:fs";
import path from "node:path";
import axios from "axios";

const GITHUB_API_BASE_URL = "https://api.github.com";
const OWNER = "Mozilla-Ocho";
const REPO = "llamafile";
const OUTPUT_DIR = "./src-tauri/bin";

const build_names = [
	// macOs
	"llamafiler-aarch64-apple-darwin",
	"llamafiler-x86_64-apple-darwin",
	"llamafiler-x86_64-apple-darwin",
	// Windows
	"llamafiler-x86_64-pc-windows-msvc.exe",
	"llamafiler-i686-pc-windows-msvc.exe",
	"llamafiler-aarch64-pc-windows-msvc.exe",
	// Linux
	"llamafiler-x86_64-unknown-linux-gnu",
	"llamafiler-aarch64-unknown-linux-gnu",
	"llamafiler-armv7-unknown-linux-gnueabihf",
	"llamafiler-i686-unknown-linux-gnu",
];

if (!fs.existsSync(OUTPUT_DIR)) {
	fs.mkdirSync(OUTPUT_DIR);
}

interface Asset {
	name: string;
	browser_download_url: string;
}

function makeExecutable(filePath: string): void {
	fs.chmodSync(filePath, "755"); // Sets the file to be readable and executable by everyone, writable by the owner
}

async function getLatestRelease(): Promise<{ assets: Asset[] }> {
	const url = `${GITHUB_API_BASE_URL}/repos/${OWNER}/${REPO}/releases/latest`;
	const response = process.env.GITHUB_TOKEN
		? await axios.get(url, {
				headers: {
					Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
				},
			})
		: await axios.get(url);
	return response.data;
}

async function downloadFile(url: string): Promise<void> {
	const outputPath = path.join(OUTPUT_DIR, "llamafiler");
	const writer = fs.createWriteStream(outputPath);

	const response = await axios({
		url,
		method: "GET",
		responseType: "stream",
	});

	response.data.pipe(writer);

	return new Promise((resolve, reject) => {
		writer.on("finish", resolve);
		writer.on("error", reject);
	});
}

async function main() {
	const force = process.argv.includes("--force");

	if (!force) {
		try {
			const files = fs.readdirSync(OUTPUT_DIR);
			if (files.includes("llamafiler")) {
				console.log("Local LLamaFiles are up to date");
				return;
			}
		} catch (error) {
			console.error(error);
		}
	}

	try {
		const latestRelease = await getLatestRelease();
		console.log("Latest release files:");

		let done = false;

		for (const asset of latestRelease.assets) {
			console.log(asset.name);
			if (done) continue;

			if (!asset.name.endsWith(".zip") && asset.name.startsWith("llamafiler")) {
				console.log(`Downloading ${asset.name}...`);
				await downloadFile(asset.browser_download_url);
				console.log(`Downloaded ${asset.name}`);
				done = true;
			}
		}

		try {
			makeExecutable(path.join(OUTPUT_DIR, "llamafiler"));
		} catch (error) {}

		const files = fs.readdirSync(OUTPUT_DIR);

		for await (const file of files) {
			if (file === "llamafiler") continue;
			const filePath = path.join(OUTPUT_DIR, file);
			fs.unlinkSync(filePath);
			console.log(`Cleaned up old file: ${filePath}`);
		}

		for await (const build_name of build_names) {
			const filePath = path.join(OUTPUT_DIR, build_name);
			fs.copyFileSync(path.join(OUTPUT_DIR, "llamafiler"), filePath);
		}

		console.log("Updated Local LLamaFiles");
	} catch (error) {
		console.error("An error occurred:", error);
	}
}

main();
