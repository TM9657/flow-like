export const CHANNEL = "flow-like-craft";
export const MAX_DOCUMENT_BYTES = 200 * 1024 * 1024;

export function validateCommand(data, session, manifest) {
	if (!data || data.channel !== CHANNEL || data.session !== session)
		return null;
	if (data.type === "open") {
		if (
			typeof data.name !== "string" ||
			data.name.length > 1024 ||
			!manifest?.extensions?.includes(
				data.name.split(".").at(-1)?.toLowerCase(),
			)
		) {
			throw new Error("This file type is not supported by this editor.");
		}
		if (
			!(data.bytes instanceof ArrayBuffer) ||
			!data.bytes.byteLength ||
			data.bytes.byteLength >
				Math.min(manifest?.maxBytes || MAX_DOCUMENT_BYTES, MAX_DOCUMENT_BYTES)
		) {
			throw new Error("Documents must be between 1 byte and 200 MiB.");
		}
		return {
			type: "open",
			name: data.name,
			bytes: data.bytes,
			readOnly: data.readOnly !== false,
		};
	}
	if (data.type === "set-read-only" && typeof data.readOnly === "boolean") {
		return { type: data.type, readOnly: data.readOnly };
	}
	if (
		data.type === "export" &&
		typeof data.requestId === "string" &&
		data.requestId.length <= 128
	) {
		if (
			typeof data.format !== "string" ||
			!/^[a-z0-9]+$/.test(data.format) ||
			data.format.length > 16
		)
			return null;
		return { type: data.type, requestId: data.requestId, format: data.format };
	}
	if (
		data.type === "saved" &&
		Number.isSafeInteger(data.revision) &&
		data.revision >= 0
	) {
		return { type: data.type, revision: data.revision };
	}
	return null;
}
