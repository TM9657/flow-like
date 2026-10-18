import contract from "../../../core/a2ui-schema/src/file-preview-event-schemas.json";

export type FilePreviewEventName = keyof typeof contract.events;

export type FilePreviewExportedFile = {
	name: string;
	size: number;
	type: string;
	url: string;
	backendUrl: string;
};

type SavedFileDetails = {
	url: string;
	signedUrls: string;
	file: FilePreviewExportedFile;
	value: FilePreviewExportedFile;
};

export type FilePreviewEventDetails = {
	save: SavedFileDetails;
	saved: SavedFileDetails;
	dirtyChange: { dirty: boolean };
	error: { message: string };
};

/** Shared with the workflow catalog so creation and later inference use the same schema. */
export function getFilePreviewEventContextSchema(
	eventName: string,
): Record<string, unknown> | undefined {
	if (!Object.hasOwn(contract.events, eventName)) return undefined;
	const definition = contract.events[eventName as FilePreviewEventName];
	return {
		$schema: contract.$schema,
		$defs: contract.$defs,
		...contract.$defs[definition as keyof typeof contract.$defs],
	};
}
