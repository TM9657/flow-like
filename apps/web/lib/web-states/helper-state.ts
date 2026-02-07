import type { IHelperState } from "@tm9657/flow-like-ui";
import type { IFileMetadata } from "@tm9657/flow-like-ui/lib";
import { type WebBackendRef, apiGet } from "./api-utils";

interface ITemporaryFileResponse {
	key: string;
	contentType: string;
	uploadUrl: string;
	uploadExpiresAt: string;
	downloadUrl: string;
	downloadExpiresAt: string;
	headUrl: string;
	deleteUrl: string;
	sizeLimitBytes?: number;
}

export class WebHelperState implements IHelperState {
	constructor(private readonly backend: WebBackendRef) {}

	async getPathMeta(folderPath: string): Promise<IFileMetadata[]> {
		return apiGet<IFileMetadata[]>(
			`helper/path-meta?path=${encodeURIComponent(folderPath)}`,
			this.backend.auth,
		);
	}

	async openFileOrFolderMenu(
		multiple: boolean,
		directory: boolean,
		recursive: boolean,
	): Promise<string[] | string | undefined> {
		// Web version: Cannot open native file dialogs
		// This would need to use HTML file input instead
		console.warn(
			"openFileOrFolderMenu is not available in web mode - use HTML file input",
		);
		return undefined;
	}

	async fileToUrl(file: File, offline?: boolean): Promise<string> {
		if (!this.backend.auth) {
			return URL.createObjectURL(file);
		}

		const response: ITemporaryFileResponse = await apiGet(
			`tmp?extension=${encodeURIComponent(file.name.split(".").pop() || "")}&filename=${encodeURIComponent(file.name)}`,
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
}

function buildContentDisposition(
	filename: string,
	disposition: "inline" | "attachment" = "inline",
): string {
	let fallback = filename
		.normalize("NFKD")
		.replace(/[^\x20-\x7E]+/g, "")
		.replace(/["\\]/g, "_")
		.trim();

	if (!fallback) {
		fallback = "file";
	}

	const encoded = encodeURIComponent(filename);
	return `${disposition}; filename="${fallback}"; filename*=UTF-8''${encoded}`;
}
