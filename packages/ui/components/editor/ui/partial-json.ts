export interface PartialJsonOptions {
	/** Include complete fields from the last open object or array in an array. */
	allowPartialArrayItems?: boolean;
}

type Parsed =
	| { state: "complete" | "partial"; value: unknown }
	| { state: "incomplete" };

const INCOMPLETE: Parsed = { state: "incomplete" };
const NUMBER = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/;
const NUMBER_PREFIX =
	/^-?(?:(?:0|[1-9]\d*)(?:\.\d*)?)?$|^-?(?:0|[1-9]\d*)(?:\.\d+)?[eE][+-]?\d*$/;
const DELIMITER = /[\s,\]}]/;
const WHITESPACE = /[ \t\r\n]/;

/**
 * Read a JSON prefix while a fenced block is streaming. Retain only complete
 * scalar values; unfinished containers may expose their already parsed fields.
 * Numbers need a following delimiter so a streamed `4` cannot appear before `40`.
 * Invalid syntax returns undefined. Settled content must still use JSON.parse.
 */
export function parsePartialJson(
	source: string,
	options: PartialJsonOptions = {},
): unknown | undefined {
	let cursor = 0;
	const invalid = () => {
		throw new Error("Invalid JSON prefix");
	};
	const whitespace = () => {
		while (cursor < source.length && WHITESPACE.test(source[cursor])) cursor++;
	};

	function string(): Parsed {
		const start = cursor++;
		while (cursor < source.length) {
			const char = source[cursor++];
			if (char === '"')
				return {
					state: "complete",
					value: JSON.parse(source.slice(start, cursor)),
				};
			if (char.charCodeAt(0) < 0x20) invalid();
			if (char !== "\\") continue;
			if (cursor === source.length) return INCOMPLETE;
			const escaped = source[cursor++];
			if ('"\\/bfnrt'.includes(escaped)) continue;
			if (escaped !== "u") invalid();
			for (let index = 0; index < 4; index++) {
				if (cursor === source.length) return INCOMPLETE;
				if (!/[\da-f]/i.test(source[cursor++])) invalid();
			}
		}
		return INCOMPLETE;
	}

	function scalar(): Parsed {
		const start = cursor;
		while (cursor < source.length && !DELIMITER.test(source[cursor])) cursor++;
		const token = source.slice(start, cursor);
		const literal = ["true", "false", "null"].find((value) =>
			value.startsWith(token),
		);
		if (cursor === source.length) {
			if (literal || NUMBER_PREFIX.test(token)) return INCOMPLETE;
			return invalid();
		}
		if (token === literal || NUMBER.test(token))
			return { state: "complete", value: JSON.parse(token) };
		return invalid();
	}

	function object(depth: number): Parsed {
		cursor++;
		const value: Record<string, unknown> = {};
		whitespace();
		if (source[cursor] === "}") {
			cursor++;
			return { state: "complete", value };
		}
		while (cursor < source.length) {
			if (source[cursor] !== '"') return invalid();
			const key = string();
			if (key.state !== "complete") return { state: "partial", value };
			whitespace();
			if (cursor === source.length) return { state: "partial", value };
			if (source[cursor++] !== ":") return invalid();
			const child = read(depth + 1);
			if (child.state !== "incomplete") {
				// Define a data property so a JSON key named __proto__ stays inert.
				Object.defineProperty(value, key.value as string, {
					value: child.value,
					writable: true,
					enumerable: true,
					configurable: true,
				});
			}
			if (child.state !== "complete") return { state: "partial", value };
			whitespace();
			if (cursor === source.length) return { state: "partial", value };
			const next = source[cursor++];
			if (next === "}") return { state: "complete", value };
			if (next !== ",") return invalid();
			whitespace();
			// A comma may be the current stream tail, but a trailing comma before }
			// is invalid and must never be repaired into a valid object.
			if (source[cursor] === "}") return invalid();
		}
		return { state: "partial", value };
	}

	function array(depth: number): Parsed {
		cursor++;
		const value: unknown[] = [];
		whitespace();
		if (source[cursor] === "]") {
			cursor++;
			return { state: "complete", value };
		}
		while (cursor < source.length) {
			const child = read(depth + 1);
			if (
				child.state === "complete" ||
				(child.state === "partial" && options.allowPartialArrayItems)
			)
				value.push(child.value);
			if (child.state !== "complete") return { state: "partial", value };
			whitespace();
			if (cursor === source.length) return { state: "partial", value };
			const next = source[cursor++];
			if (next === "]") return { state: "complete", value };
			if (next !== ",") return invalid();
			whitespace();
			if (source[cursor] === "]") return invalid();
		}
		return { state: "partial", value };
	}

	function read(depth: number): Parsed {
		if (depth > 128) return invalid();
		whitespace();
		if (cursor === source.length) return INCOMPLETE;
		if (source[cursor] === "{") return object(depth);
		if (source[cursor] === "[") return array(depth);
		if (source[cursor] === '"') return string();
		return scalar();
	}

	try {
		const parsed = read(0);
		whitespace();
		if (cursor !== source.length || parsed.state === "incomplete")
			return undefined;
		return parsed.value;
	} catch {
		return undefined;
	}
}
