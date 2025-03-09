import fs from "fs";
import path from "path";
import AdmZip from "adm-zip";
import axios from "axios";
const zlib = require("zlib");
const tar = require("tar");

const GITHUB_API_BASE_URL = "https://api.github.com";
const OWNER = "jgm";
const REPO = "pandoc";
const OUTPUT_DIR = "./src-tauri/bin";

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

function extractFileFromZip(
	zipFilePath: string,
	fileToExtract: string,
	outputPath: string,
) {
	zipFilePath = path.join(OUTPUT_DIR, zipFilePath);
	try {
		const zip = new AdmZip(zipFilePath);

		// Überprüfen, ob die Datei in der ZIP-Datei existiert
		const zipEntries = zip.getEntries();
		const entry = zipEntries.find((entry) => {
			const fileName = entry.entryName.split("/").pop();
			return fileName === fileToExtract;
		});

		if (entry) {
			// Extrahieren der Datei
			zip.extractEntryTo(entry, OUTPUT_DIR, false, true, false, outputPath);
			makeExecutable(path.join(OUTPUT_DIR, outputPath));
			console.log(`File '${fileToExtract}' extracted to '${outputPath}'.`);
		} else {
			console.log(`File '${fileToExtract}' not found.`);
		}
	} catch (err) {
		console.error(`Error extracting.. ${err}`);
	}
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

async function downloadFile(url: string, fileName: string): Promise<void> {
	const outputPath = path.join(OUTPUT_DIR, fileName);
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

async function extractFileFromTarGz(
	zipFilePath: string,
	fileToExtract: string,
	outputPath: string,
) {
	zipFilePath = path.join(OUTPUT_DIR, zipFilePath);
	outputPath = path.join(OUTPUT_DIR, outputPath);
	return new Promise((resolve, reject) => {
		try {
			fs.createReadStream(zipFilePath)
				.pipe(zlib.createGunzip()) // Entpacken der .gz-Kompression
				.pipe(
					tar.t({
						onentry: (entry: any) => {
							const fileName = entry.path.split("/").pop();
							if (fileName === fileToExtract) {
								entry.pipe(fs.createWriteStream(outputPath));
								console.log(
									`File '${fileToExtract}' successfully extracted to '${outputPath}'.`,
								);
							}
						},
					}),
				)
				.on("error", (err: any) =>
					console.error(`Error extracting file: ${err}`),
				)
				.on("end", () => {
					if (!fs.existsSync(outputPath)) {
						console.log(`File '${fileToExtract}' not found.`);
						reject(new Error(`File '${fileToExtract}' not found.`));
					}
					resolve(true);
				});
		} catch (err) {
			console.error(`Error extracting: ${err}`);
		}
	});
}

function listFiles(dir: string) {
	const files = fs.readdirSync(dir);
	for (const file of files) {
		console.log(file);
	}
}

async function main() {
	const force = process.argv.includes("--force");

	if (!force) {
		try {
			const files = fs.readdirSync(OUTPUT_DIR);
			let hasPandoc = false;
			for (const file of files) {
				if (file.startsWith("pandoc")) {
					hasPandoc = true;
				}
			}
			if (hasPandoc) {
				console.log("Local Pandoc binaries are up to date");
				return;
			}
		} catch (error) {
			console.error(error);
		}
	}

	try {
		const latestRelease = await getLatestRelease();
		console.log("Latest release files:");

		for (const asset of latestRelease.assets) {
			if (asset.name.endsWith("arm64-macOS.zip")) {
				console.log(`Downloading ${asset.name}...`);
				await downloadFile(asset.browser_download_url, "arm-macos.zip");
				extractFileFromZip(
					"arm-macos.zip",
					"pandoc",
					"pandoc-aarch64-apple-darwin",
				);
				fs.unlinkSync(path.join(OUTPUT_DIR, "arm-macos.zip"));
				console.log(`Downloaded ${asset.name}`);
			}
			if (asset.name.endsWith("x86_64-macOS.zip")) {
				console.log(`Downloading ${asset.name}...`);
				await downloadFile(asset.browser_download_url, "amd-macos.zip");
				extractFileFromZip(
					"amd-macos.zip",
					"pandoc",
					"pandoc-x86_64-apple-darwin",
				);
				fs.unlinkSync(path.join(OUTPUT_DIR, "amd-macos.zip"));
				console.log(`Downloaded ${asset.name}`);
			}
			if (asset.name.endsWith("windows-x86_64.zip")) {
				console.log(`Downloading ${asset.name}...`);
				await downloadFile(asset.browser_download_url, "amd-windows.zip");
				extractFileFromZip(
					"amd-windows.zip",
					"pandoc.exe",
					"pandoc-x86_64-pc-windows-msvc.exe",
				);
				fs.unlinkSync(path.join(OUTPUT_DIR, "amd-windows.zip"));
				console.log(`Downloaded ${asset.name}`);
			}
			if (asset.name.endsWith("linux-amd64.tar.gz")) {
				console.log(`Downloading ${asset.name}...`);
				await downloadFile(asset.browser_download_url, "amd-linux.tar.gz");
				await extractFileFromTarGz(
					"amd-linux.tar.gz",
					"pandoc",
					"pandoc-x86_64-unknown-linux-gnu",
				);
				fs.unlinkSync(path.join(OUTPUT_DIR, "amd-linux.tar.gz"));
				console.log(`Downloaded ${asset.name}`);
			}
			if (asset.name.endsWith("linux-arm64.tar.gz")) {
				console.log(`Downloading ${asset.name}...`);
				await downloadFile(asset.browser_download_url, "arm-linux.tar.gz");
				await extractFileFromTarGz(
					"arm-linux.tar.gz",
					"pandoc",
					"pandoc-aarch64-unknown-linux-gnu",
				);
				fs.unlinkSync(path.join(OUTPUT_DIR, "arm-linux.tar.gz"));
				console.log(`Downloaded ${asset.name}`);
			}
		}
	} catch (error) {
		console.error("An error occurred:", error);
	}
}

main();
