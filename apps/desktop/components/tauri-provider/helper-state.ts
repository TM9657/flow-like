import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { temporaryFilesDb, type IFileMetadata, type IHelperState } from "@tm9657/flow-like-ui";
import type { TauriBackend } from "../tauri-provider";
import { get } from "../../lib/api";
import { appCacheDir } from "@tauri-apps/api/path";
import { writeFile, mkdir } from "@tauri-apps/plugin-fs"
import { createId } from "@paralleldrive/cuid2";

interface ITemporaryFileResponse {
	key: string,
	contentType: string,
	uploadUrl: string,
	uploadExpiresAt: string,
	downloadUrl: string,
	downloadExpiresAt: string,
	headUrl: string,
	deleteUrl: string,
	sizeLimitBytes?: number,
}

export class HelperState implements IHelperState {
	constructor(private readonly backend: TauriBackend) { }

	async getPathMeta(path: string): Promise<IFileMetadata[]> {
		return await invoke("get_path_meta", {
			path: path,
		});
	}
	async openFileOrFolderMenu(
		multiple: boolean,
		directory: boolean,
		recursive: boolean,
	): Promise<string[] | string | undefined> {
		return (
			(await open({
				multiple: multiple,
				directory: directory,
				recursive: recursive,
			})) ?? undefined
		);
	}

	async fileToUrl(file: File, offline: boolean = false): Promise<string> {
		if (!offline) {
			if (!this.backend.profile || !this.backend.auth) {
				throw new Error("Profile or auth not set");
			}

			const response: ITemporaryFileResponse = await get(
				this.backend.profile,
				`tmp?extension=${encodeURIComponent(file.name.split('.').pop() || '')}`,
				this.backend.auth,
			);

			await fetch(response.uploadUrl, {
				method: "PUT",
				headers: {
					"Content-Type": file.type,
					"Content-Disposition": buildContentDisposition(file.name, "inline"),
				},
				body: file,
			});

			return response.downloadUrl;
		}

		const cacheDir = await appCacheDir();
		const fileId = createId();

		const extension = file.name.split('.').pop();

		try {
			await mkdir(`${cacheDir}/chat`, { recursive: true });
		}catch(e) {

		}

		const tmpPath = `${cacheDir}/chat/${fileId}.${extension}`;

		await writeFile(tmpPath, file.stream());

		const postProcessedPath = await invoke<string>("post_process_local_file", {
			file: tmpPath,
		});

		const hash = postProcessedPath.split('/').pop() || fileId;

		await temporaryFilesDb.temporaryFiles.put({
			id: fileId,
			fileName: file.name,
			size: file.size,
			hash: hash,
			createdAt: Date.now(),
		})

		return convertFileSrc(postProcessedPath);
	}
}

function buildContentDisposition(
	filename: string,
	disposition: "inline" | "attachment" = "inline",
): string {
	// 1. Fallback ASCII filename (for old/strict user agents)
	// - Normalize to decompose accents
	// - Strip non-ASCII
	// - Replace quotes/backslashes
	let fallback = filename
		.normalize("NFKD")
		.replace(/[^\x20-\x7E]+/g, "")   // remove non-ASCII
		.replace(/["\\]/g, "_")
		.trim();

	if (!fallback) {
		fallback = "file";
	}

	// 2. RFC 5987 / RFC 6266 UTF-8 filename*
	const encoded = encodeURIComponent(filename);

	return `${disposition}; filename="${fallback}"; filename*=UTF-8''${encoded}`;
}