export type JsonDocumentResult =
	| { ok: true; value: unknown }
	| { ok: false; error: string };

export type FormattedHomeLayoutJsonResult =
	| { ok: true; json: string }
	| { ok: false; error: string };

// serde_json allows 128 nested containers. Leave room for the API request and desktop
// Settings -> profiles -> profile -> hub_profile envelopes around a saved Home layout.
export const MAX_HOME_JSON_DEPTH = 64;
export const MAX_HOME_JSON_VALUES = 65_536;
export const MAX_HOME_JSON_TEXT_BYTES = 1024 * 1024;

const textEncoder = new TextEncoder();
const depthError = `JSON must not exceed ${MAX_HOME_JSON_DEPTH} nested objects or arrays.`;
const valueCountError = `JSON must not contain more than ${MAX_HOME_JSON_VALUES} values.`;
const unicodeError =
	"JSON keys and text must contain valid Unicode. Replace any unpaired surrogate characters.";
const finiteNumberError =
	"JSON numbers must be finite. Replace values such as 1e400 with a finite number.";

function quotedStringBytes(value: string): number | null {
	if (value.length > MAX_HOME_JSON_TEXT_BYTES)
		return MAX_HOME_JSON_TEXT_BYTES + 1;
	let bytes = 2;
	for (let index = 0; index < value.length; index++) {
		const code = value.charCodeAt(index);
		if (code >= 0xd800 && code <= 0xdbff) {
			const next = value.charCodeAt(++index);
			if (!(next >= 0xdc00 && next <= 0xdfff)) return null;
			bytes += 4;
		} else if (code >= 0xdc00 && code <= 0xdfff) return null;
		else if (code === 0x22 || code === 0x5c) bytes += 2;
		else if (code < 0x20)
			bytes += [0x08, 0x09, 0x0a, 0x0c, 0x0d].includes(code) ? 2 : 6;
		else bytes += code < 0x80 ? 1 : code < 0x800 ? 2 : 3;
	}
	return bytes;
}

/** Bound traversal and pretty-print expansion before calling the native stringifier. */
function documentError(value: unknown): string | null {
	const pending: Array<{ value: unknown; depth: number }> = [
		{ value, depth: 0 },
	];
	let values = 1;
	let prettyBytes = 0;
	while (pending.length) {
		const item = pending.pop();
		if (!item) break;
		const current = item.value;
		if (current === null) prettyBytes += 4;
		else if (typeof current === "string") {
			const bytes = quotedStringBytes(current);
			if (bytes === null) return unicodeError;
			prettyBytes += bytes;
		} else if (typeof current === "number") {
			if (!Number.isFinite(current)) return finiteNumberError;
			prettyBytes += JSON.stringify(current).length;
		} else if (typeof current === "boolean") prettyBytes += current ? 4 : 5;
		else if (typeof current === "object") {
			const depth = item.depth + 1;
			if (depth > MAX_HOME_JSON_DEPTH) return depthError;
			const array = Array.isArray(current);
			if (
				!array &&
				Object.getPrototypeOf(current) !== Object.prototype &&
				Object.getPrototypeOf(current) !== null
			)
				return "JSON objects must contain plain data.";
			const keys = array ? null : Object.keys(current);
			const length = array ? current.length : (keys?.length ?? 0);
			if (values + length > MAX_HOME_JSON_VALUES) return valueCountError;
			values += length;
			prettyBytes += 2;
			let entries = 0;
			for (let index = 0; index < length; index++) {
				const key = keys?.[index];
				const child = array
					? current[index]
					: (current as Record<string, unknown>)[key as string];
				// Optional layout fields are undefined in memory and omitted by JSON.stringify.
				if (!array && child === undefined) continue;
				if (key !== undefined) {
					const bytes = quotedStringBytes(key);
					if (bytes === null) return unicodeError;
					prettyBytes += bytes + 2;
				}
				prettyBytes += 1 + depth * 2 + (entries > 0 ? 1 : 0);
				entries++;
				pending.push({ value: child === undefined ? null : child, depth });
			}
			if (entries > 0) prettyBytes += 1 + (depth - 1) * 2;
		} else
			return "JSON must contain only objects, arrays, text, finite numbers, booleans, or null.";
		if (prettyBytes > MAX_HOME_JSON_TEXT_BYTES)
			return "Formatted JSON exceeds the 1 MiB editor limit. Reduce the document size or nesting.";
	}
	return null;
}

export function parseJsonDocument(source: string): JsonDocumentResult {
	try {
		// Check code units first so encoding a huge paste does not allocate another huge buffer.
		if (
			source.length > MAX_HOME_JSON_TEXT_BYTES ||
			textEncoder.encode(source).byteLength > MAX_HOME_JSON_TEXT_BYTES
		)
			return { ok: false, error: "JSON exceeds the 1 MiB editor input limit." };
		let depth = 0;
		let quoted = false;
		let escaped = false;
		let stringStart = 0;
		for (let index = 0; index < source.length; index++) {
			const character = source[index];
			if (quoted) {
				if (escaped) escaped = false;
				else if (character === "\\") escaped = true;
				else if (character === '"') {
					quoted = false;
					// Validate every token, including values later overwritten by a duplicate key.
					// serde rejects invalid Unicode or numbers before it can replace that entry.
					const text: string = JSON.parse(source.slice(stringStart, index + 1));
					if (quotedStringBytes(text) === null)
						return { ok: false, error: unicodeError };
				}
			} else if (character === '"') {
				quoted = true;
				stringStart = index;
			} else if (character === "[" || character === "{") {
				if (++depth > MAX_HOME_JSON_DEPTH)
					return { ok: false, error: depthError };
			} else if (character === "]" || character === "}") depth--;
			else if (character === "-" || (character >= "0" && character <= "9")) {
				let end = index + 1;
				while (end < source.length && /[\d.eE+-]/.test(source[end])) end++;
				if (!Number.isFinite(Number(source.slice(index, end))))
					return { ok: false, error: finiteNumberError };
				index = end - 1;
			}
		}
		const value: unknown = JSON.parse(source);
		const error = documentError(value);
		return error ? { ok: false, error } : { ok: true, value };
	} catch (error) {
		return {
			ok: false,
			error: `Invalid JSON: ${error instanceof Error ? error.message : "Check the document syntax."}`,
		};
	}
}

export function formatJsonDocument(
	value: unknown,
): FormattedHomeLayoutJsonResult {
	try {
		const error = documentError(value);
		if (error) return { ok: false, error };
		return { ok: true, json: JSON.stringify(value, null, 2) };
	} catch {
		return { ok: false, error: "Could not format this JSON document." };
	}
}
