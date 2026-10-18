import { validateMcpRequest } from "./mcp.mjs";

export const CHANNEL = "flow-like-wordcraft";
export const MAX_DOCUMENT_BYTES = 50 * 1024 * 1024;

export function validateCommand(data, session) {
	if (!data || data.channel !== CHANNEL || data.session !== session)
		return null;
	if (
		data.type === "mcp-cancel" &&
		typeof data.requestId === "string" &&
		data.requestId.length <= 128
	)
		return { type: data.type, requestId: data.requestId };
	if (data.type === "mcp-request") return validateMcpRequest(data);
	if (data.type === "open" || data.type === "create") {
		if (
			typeof data.name !== "string" ||
			data.name.length > 1024 ||
			!/\.docx$/i.test(data.name)
		) {
			throw new Error("Only DOCX documents are supported.");
		}
		if (data.type === "create")
			return {
				type: "create",
				name: data.name,
				readOnly: data.readOnly !== false,
			};
		if (
			!(data.bytes instanceof ArrayBuffer) ||
			!data.bytes.byteLength ||
			data.bytes.byteLength > MAX_DOCUMENT_BYTES
		) {
			throw new Error("Documents must be between 1 byte and 50 MiB.");
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
		return {
			type: data.type,
			requestId: data.requestId,
			...(data.purpose === "download" ? { purpose: "download" } : {}),
		};
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
