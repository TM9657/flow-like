/**
 * A small, dependency-free XML reader for importer inputs (BPMN and friends).
 *
 * The browser has `DOMParser`, the bun test runner does not, and pulling a
 * parser library into `packages/ui` for one importer is not worth a lockfile
 * change. Tool-exported XML is well-formed, so this handles the well-formed
 * subset — elements, attributes, text, CDATA, comments, processing
 * instructions, a DOCTYPE to skip, and the predefined + numeric entities —
 * and throws on anything else rather than guessing.
 */

export interface XmlElement {
	/** Local name with any namespace prefix stripped: `bpmn:task` → `task`. */
	name: string;
	/** Namespace prefix as written, or `null` when there was none. */
	prefix: string | null;
	/** Attributes keyed by their qualified name exactly as written. */
	attrs: Record<string, string>;
	children: XmlElement[];
	/** Direct text and CDATA content, concatenated and entity-decoded. */
	text: string;
}

export class XmlParseError extends Error {
	constructor(
		message: string,
		public readonly offset: number,
	) {
		super(`${message} (offset ${offset})`);
		this.name = "XmlParseError";
	}
}

const NAME_START = /[A-Za-z_:À-￿]/;
const NAME_CHAR = /[A-Za-z0-9_:.\-·À-￿]/;

const PREDEFINED_ENTITIES: Record<string, string> = {
	lt: "<",
	gt: ">",
	amp: "&",
	quot: '"',
	apos: "'",
};

/** A code point `String.fromCodePoint` accepts and that is legal in XML text. */
function isUsableCodePoint(code: number): boolean {
	return (
		Number.isInteger(code) &&
		code > 0 &&
		code <= 0x10ffff &&
		!(code >= 0xd800 && code <= 0xdfff)
	);
}

export function decodeXmlEntities(value: string): string {
	if (!value.includes("&")) return value;
	return value.replace(
		/&(#x[0-9a-fA-F]+|#[0-9]+|[A-Za-z][A-Za-z0-9]*);/g,
		(match, body: string) => {
			// An unusable reference is left as written rather than throwing: it is a
			// flaw in one label, not a reason to reject the document.
			if (body.startsWith("#x") || body.startsWith("#X")) {
				const code = Number.parseInt(body.slice(2), 16);
				return isUsableCodePoint(code) ? String.fromCodePoint(code) : match;
			}
			if (body.startsWith("#")) {
				const code = Number.parseInt(body.slice(1), 10);
				return isUsableCodePoint(code) ? String.fromCodePoint(code) : match;
			}
			return PREDEFINED_ENTITIES[body] ?? match;
		},
	);
}

function splitQName(qname: string): { prefix: string | null; name: string } {
	const colon = qname.indexOf(":");
	if (colon < 0) return { prefix: null, name: qname };
	return { prefix: qname.slice(0, colon), name: qname.slice(colon + 1) };
}

class Scanner {
	pos = 0;
	constructor(readonly src: string) {}

	peek(offset = 0): string {
		return this.src[this.pos + offset] ?? "";
	}

	startsWith(token: string): boolean {
		return this.src.startsWith(token, this.pos);
	}

	skipWhitespace(): void {
		while (this.pos < this.src.length && /\s/.test(this.src[this.pos])) {
			this.pos += 1;
		}
	}

	/** Advances past `token`, which must appear at or after the cursor. */
	skipPast(token: string, what: string): void {
		const end = this.src.indexOf(token, this.pos);
		if (end < 0) throw new XmlParseError(`Unterminated ${what}`, this.pos);
		this.pos = end + token.length;
	}

	readUntil(token: string, what: string): string {
		const end = this.src.indexOf(token, this.pos);
		if (end < 0) throw new XmlParseError(`Unterminated ${what}`, this.pos);
		const out = this.src.slice(this.pos, end);
		this.pos = end + token.length;
		return out;
	}

	readName(): string {
		const start = this.pos;
		if (!NAME_START.test(this.peek())) {
			throw new XmlParseError("Expected a name", this.pos);
		}
		this.pos += 1;
		while (this.pos < this.src.length && NAME_CHAR.test(this.src[this.pos])) {
			this.pos += 1;
		}
		return this.src.slice(start, this.pos);
	}

	readQuoted(): string {
		const quote = this.peek();
		if (quote !== '"' && quote !== "'") {
			throw new XmlParseError("Expected a quoted attribute value", this.pos);
		}
		this.pos += 1;
		const raw = this.readUntil(quote, "attribute value");
		// Literal whitespace in an attribute value normalizes to a space (XML 1.0
		// §3.3.3), so a wrapped attribute does not become a multi-line node name.
		// Character references are decoded after, so `&#10;` still yields a newline.
		return decodeXmlEntities(raw.replace(/[\t\n]/g, " "));
	}
}

function skipDoctype(scanner: Scanner): void {
	scanner.pos += "<!DOCTYPE".length;
	let depth = 0;
	let quote = "";
	while (scanner.pos < scanner.src.length) {
		const char = scanner.peek();
		// A SYSTEM/PUBLIC literal may contain `>` and brackets, so nothing inside
		// one counts towards the internal subset or ends the declaration.
		if (quote) {
			if (char === quote) quote = "";
		} else if (char === '"' || char === "'") {
			quote = char;
		} else if (char === "[") {
			depth += 1;
		} else if (char === "]") {
			depth -= 1;
		} else if (char === ">" && depth <= 0) {
			scanner.pos += 1;
			return;
		}
		scanner.pos += 1;
	}
	throw new XmlParseError("Unterminated DOCTYPE", scanner.pos);
}

/**
 * Skips everything that can sit between elements at the document level or
 * inside one without being content: whitespace, comments, PIs, a DOCTYPE.
 */
function skipMisc(scanner: Scanner, allowDoctype: boolean): void {
	for (;;) {
		scanner.skipWhitespace();
		if (scanner.startsWith("<!--")) {
			scanner.pos += 4;
			scanner.skipPast("-->", "comment");
		} else if (scanner.startsWith("<?")) {
			scanner.pos += 2;
			scanner.skipPast("?>", "processing instruction");
		} else if (allowDoctype && scanner.startsWith("<!DOCTYPE")) {
			skipDoctype(scanner);
		} else {
			return;
		}
	}
}

function readAttributes(scanner: Scanner): Record<string, string> {
	const attrs: Record<string, string> = {};
	for (;;) {
		scanner.skipWhitespace();
		const char = scanner.peek();
		if (char === "" || char === ">" || char === "/") return attrs;
		const name = scanner.readName();
		scanner.skipWhitespace();
		if (scanner.peek() !== "=") {
			throw new XmlParseError(`Attribute "${name}" has no value`, scanner.pos);
		}
		scanner.pos += 1;
		scanner.skipWhitespace();
		attrs[name] = scanner.readQuoted();
	}
}

/** Guards the recursive descent so a hostile document throws instead of overflowing. */
const MAX_DEPTH = 256;

function readElement(scanner: Scanner, depth = 0): XmlElement {
	if (depth > MAX_DEPTH) {
		throw new XmlParseError(
			`Elements nested deeper than ${MAX_DEPTH}`,
			scanner.pos,
		);
	}
	if (scanner.peek() !== "<") {
		throw new XmlParseError("Expected an element", scanner.pos);
	}
	scanner.pos += 1;
	const qname = scanner.readName();
	const { prefix, name } = splitQName(qname);
	const attrs = readAttributes(scanner);
	const element: XmlElement = { name, prefix, attrs, children: [], text: "" };

	if (scanner.startsWith("/>")) {
		scanner.pos += 2;
		return element;
	}
	if (scanner.peek() !== ">") {
		throw new XmlParseError(`Malformed start tag <${qname}>`, scanner.pos);
	}
	scanner.pos += 1;

	const textParts: string[] = [];
	for (;;) {
		if (scanner.pos >= scanner.src.length) {
			throw new XmlParseError(`Unclosed element <${qname}>`, scanner.pos);
		}
		if (scanner.startsWith("</")) {
			scanner.pos += 2;
			const closing = scanner.readName();
			if (closing !== qname) {
				throw new XmlParseError(
					`Mismatched closing tag </${closing}> for <${qname}>`,
					scanner.pos,
				);
			}
			scanner.skipWhitespace();
			if (scanner.peek() !== ">") {
				throw new XmlParseError(
					`Malformed closing tag </${qname}>`,
					scanner.pos,
				);
			}
			scanner.pos += 1;
			element.text = textParts.join("");
			return element;
		}
		if (scanner.startsWith("<![CDATA[")) {
			scanner.pos += 9;
			textParts.push(scanner.readUntil("]]>", "CDATA section"));
			continue;
		}
		if (scanner.startsWith("<!--")) {
			scanner.pos += 4;
			scanner.skipPast("-->", "comment");
			continue;
		}
		if (scanner.startsWith("<?")) {
			scanner.pos += 2;
			scanner.skipPast("?>", "processing instruction");
			continue;
		}
		if (scanner.peek() === "<") {
			element.children.push(readElement(scanner, depth + 1));
			continue;
		}
		const next = scanner.src.indexOf("<", scanner.pos);
		const end = next < 0 ? scanner.src.length : next;
		textParts.push(decodeXmlEntities(scanner.src.slice(scanner.pos, end)));
		scanner.pos = end;
	}
}

/** Parses a whole document and returns its root element. */
export function parseXml(input: string): XmlElement {
	// A BOM is not content, and every line break normalizes to \n (XML 1.0
	// §2.11) so CRLF documentation does not carry \r into node comments.
	const src = (input.charCodeAt(0) === 0xfeff ? input.slice(1) : input).replace(
		/\r\n?/g,
		"\n",
	);
	const scanner = new Scanner(src);
	skipMisc(scanner, true);
	if (scanner.pos >= src.length) {
		throw new XmlParseError("Document has no root element", scanner.pos);
	}
	const root = readElement(scanner);
	skipMisc(scanner, false);
	if (scanner.pos < src.length) {
		throw new XmlParseError("Content after the root element", scanner.pos);
	}
	return root;
}

/** Cheap check before parsing: does this look like an XML document at all? */
export function looksLikeXml(input: string): boolean {
	const head = input.slice(0, 512).replace(/^﻿/, "").trimStart();
	return head.startsWith("<");
}

/**
 * Attribute lookup by local name. `id` matches only an unprefixed `id`;
 * passing a prefix (`"camunda"`) matches `camunda:assignee`. With
 * `anyPrefix`, the first attribute whose local name matches wins.
 */
export function attr(
	element: XmlElement,
	localName: string,
	options: { prefix?: string; anyPrefix?: boolean } = {},
): string | undefined {
	if (options.prefix) return element.attrs[`${options.prefix}:${localName}`];
	const plain = element.attrs[localName];
	if (plain !== undefined || !options.anyPrefix) return plain;
	for (const [key, value] of Object.entries(element.attrs)) {
		if (splitQName(key).name === localName) return value;
	}
	return undefined;
}

/** Direct children whose local name matches, in document order. */
export function childrenNamed(
	element: XmlElement,
	localName: string,
): XmlElement[] {
	return element.children.filter((child) => child.name === localName);
}

export function firstChild(
	element: XmlElement,
	localName: string,
): XmlElement | undefined {
	return element.children.find((child) => child.name === localName);
}

/** Every element in the subtree, depth-first, the element itself first. */
export function* walk(element: XmlElement): Generator<XmlElement> {
	yield element;
	for (const child of element.children) yield* walk(child);
}

/** Text with surrounding whitespace removed; `undefined` when empty. */
export function textOf(element: XmlElement | undefined): string | undefined {
	const trimmed = element?.text.trim();
	return trimmed ? trimmed : undefined;
}
