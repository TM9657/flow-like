export const WORDCRAFT_CHANNEL = "flow-like-wordcraft";
export const DOCX_MIME =
	"application/vnd.openxmlformats-officedocument.wordprocessingml.document";
export const MAX_DOCX_BYTES = 50 * 1024 * 1024;

export type WordcraftMessage =
	| { type: "ready" }
	| { type: "opened" }
	| { type: "save-request" }
	| { type: "dirty"; dirty: boolean }
	| { type: "error"; error: string; requestId?: string }
	| {
			type: "exported";
			requestId: string;
			bytes: ArrayBuffer;
			revision: number;
	  };

export function readWordcraftMessage(
	event: MessageEvent,
	frame: Window | null,
	origin: string,
	session: string,
): WordcraftMessage | null {
	if (!frame || event.source !== frame || event.origin !== origin) return null;
	const data = event.data;
	if (
		!data ||
		typeof data !== "object" ||
		data.channel !== WORDCRAFT_CHANNEL ||
		data.session !== session
	)
		return null;
	switch (data.type) {
		case "ready":
		case "opened":
		case "save-request":
			return { type: data.type };
		case "dirty":
			return typeof data.dirty === "boolean"
				? { type: "dirty", dirty: data.dirty }
				: null;
		case "error":
			return typeof data.error === "string"
				? {
						type: "error",
						error: data.error.slice(0, 1000),
						requestId:
							typeof data.requestId === "string" ? data.requestId : undefined,
					}
				: null;
		case "exported":
			if (
				typeof data.requestId !== "string" ||
				!(data.bytes instanceof ArrayBuffer) ||
				data.bytes.byteLength === 0 ||
				data.bytes.byteLength > MAX_DOCX_BYTES ||
				!Number.isSafeInteger(data.revision) ||
				data.revision < 0
			)
				return null;
			return {
				type: "exported",
				requestId: data.requestId,
				bytes: data.bytes,
				revision: data.revision,
			};
		default:
			return null;
	}
}
