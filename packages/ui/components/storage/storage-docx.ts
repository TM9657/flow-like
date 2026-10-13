import { storageDisplayName } from "../../lib/storage-tree";
import { basename } from "../builder/asset-path";
import { DOCX_MIME } from "./wordcraft-protocol";

export function isStorageDocx(filename: string): boolean {
	return /\.docx$/i.test(storageDisplayName(filename));
}

export function createStorageDocx(file: File, location: string): File {
	// Listed names are addressing strings. Decoding here would turn %2F into
	// a path separator or decode a literal percent escape twice on the backend.
	return new File([file], basename(location), { type: DOCX_MIME });
}
