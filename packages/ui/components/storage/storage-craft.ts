import { basename } from "../builder/asset-path";
import type { CraftViewer } from "./craft-viewers";

export function storageCraftSaveName(
	location: string,
	viewer: CraftViewer,
	conversionId: string,
): string {
	const original = basename(location);
	return viewer.extension === viewer.outputExtension
		? original
		: `${original.replace(/\.[^.]+$/, "")}-converted-${conversionId}.${viewer.outputExtension}`;
}

export function createStorageCraftFile(
	file: File,
	location: string,
	viewer: CraftViewer,
	conversionId: string,
): File {
	// Keep addressing escapes intact when replacing the listed object.
	const name = storageCraftSaveName(location, viewer, conversionId);
	return new File([file], name, { type: viewer.mime });
}
