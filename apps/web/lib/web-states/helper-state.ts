import type { IHelperState } from "@tm9657/flow-like-ui";
import type { IFileMetadata } from "@tm9657/flow-like-ui/lib";
import { type WebBackendRef, apiGet } from "./api-utils";

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
		// Web version: Create object URL for the file
		return URL.createObjectURL(file);
	}
}
