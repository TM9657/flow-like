import type { IFileMetadata } from "../../lib";

export interface IHelperState {
	getPathMeta(folderPath: string): Promise<IFileMetadata[]>;
	openFileOrFolderMenu(
		multiple: boolean,
		directory: boolean,
		recursive: boolean,
	): Promise<string[] | string | undefined>;

	/**
	 * Converts a file to a URL.
	 * @param file The file to convert.
	 * @param offline Whether to use offline storage (optional).
	 */
	fileToUrl(file: File, offline?: boolean): Promise<string>;
}
