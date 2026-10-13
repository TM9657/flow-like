export const WORDCRAFT_CHANNEL = "flow-like-wordcraft";
export const DOCX_MIME =
	"application/vnd.openxmlformats-officedocument.wordprocessingml.document";
export const MAX_DOCX_BYTES = 50 * 1024 * 1024;

export type WordcraftMessage =
	| { type: "ready" }
	| { type: "opened"; warnings?: string[]; previewOnly?: boolean }
	| { type: "save-request" }
	| { type: "dirty"; dirty: boolean }
	| { type: "error"; error: string; requestId?: string }
	| {
			type: "exported";
			requestId: string;
			bytes: ArrayBuffer;
			revision: number;
			warnings?: string[];
	  };

export function readWordcraftMessage(
	event: MessageEvent,
	frame: Window | null,
	origin: string,
	session: string,
	channel = WORDCRAFT_CHANNEL,
	maxBytes = MAX_DOCX_BYTES,
): WordcraftMessage | null {
	if (!frame || event.source !== frame || event.origin !== origin) return null;
	const data = event.data;
	if (
		!data ||
		typeof data !== "object" ||
		data.channel !== channel ||
		data.session !== session
	)
		return null;
	const warnings = Array.isArray(data.warnings)
		? data.warnings
				.filter((value: unknown): value is string => typeof value === "string")
				.slice(0, 8)
				.map((value: string) => value.slice(0, 500))
		: undefined;
	switch (data.type) {
		case "ready":
		case "save-request":
			return { type: data.type };
		case "opened":
			return {
				type: "opened",
				...(warnings?.length ? { warnings } : {}),
				...(data.previewOnly === true ? { previewOnly: true } : {}),
			};
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
				data.bytes.byteLength > maxBytes ||
				!Number.isSafeInteger(data.revision) ||
				data.revision < 0
			)
				return null;
			return {
				type: "exported",
				requestId: data.requestId,
				bytes: data.bytes,
				revision: data.revision,
				...(warnings?.length ? { warnings } : {}),
			};
		default:
			return null;
	}
}
