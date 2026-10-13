const ATTRIBUTES = new Set([
	"src",
	"alt",
	"caption",
	"credit",
	"license",
	"focalX",
	"focalY",
	"rightsExpiresAt",
	"width",
	"align",
	"title",
]);

/** Pending images use the same protocols as rich image cards. */
export function safeStreamingImageUrl(value: unknown): string | undefined {
	if (typeof value !== "string" || !value.trim()) return undefined;
	try {
		const protocol = new URL(value, "https://editor.invalid/").protocol;
		if (["http:", "https:", "blob:", "storage:"].includes(protocol))
			return value;
		if (
			/^data:image\/(?:png|jpeg|jpg|webp|gif);base64,[a-z\d+/=]+$/i.test(value)
		)
			return value;
	} catch {
		return undefined;
	}
	return undefined;
}

type ImageTail = { end: number } | { attributes: string[] } | null;

function readImageTail(source: string, start: number): ImageTail {
	let cursor = start + 4;
	const attributes = new Map<string, string>();
	const pending = () => ({ attributes: [...attributes.values()] });
	while (cursor < source.length) {
		const beforeWhitespace = cursor;
		while (/\s/.test(source[cursor] ?? "") && cursor < source.length) cursor++;
		if (cursor === source.length) return pending();
		if (source[cursor] === ">") return { end: cursor + 1 };
		if (source[cursor] === "/") {
			if (cursor + 1 === source.length) return pending();
			return source[cursor + 1] === ">" ? { end: cursor + 2 } : null;
		}
		if (cursor === beforeWhitespace) return null;
		const match = /^[A-Za-z_:][A-Za-z\d_.:-]*/.exec(source.slice(cursor));
		if (!match) return null;
		const name = match[0];
		cursor += name.length;
		while (/\s/.test(source[cursor] ?? "") && cursor < source.length) cursor++;
		if (cursor === source.length) return pending();
		if (source[cursor++] !== "=") return null;
		while (/\s/.test(source[cursor] ?? "") && cursor < source.length) cursor++;
		if (cursor === source.length) return pending();
		const quote = source[cursor++];
		if (quote !== '"' && quote !== "'") return null;
		const valueStart = cursor;
		const close = source.indexOf(quote, cursor);
		if (close < 0) return pending();
		cursor = close + 1;
		if (ATTRIBUTES.has(name))
			attributes.set(
				name,
				`${name}=${quote}${source.slice(valueStart, close).replaceAll("<", "&lt;").replaceAll(">", "&gt;")}${quote}`,
			);
	}
	return pending();
}

/**
 * Build display-only Markdown for a trailing inert image tag. Closed tags and
 * code stay byte-identical. Never publish the normalized tail to a settled cache.
 */
export function normalizeStreamingEditorialImage(markdown: string): {
	markdown: string;
	pending: boolean;
} {
	const unchanged = { markdown, pending: false };
	if (!markdown.includes("<img")) return unchanged;
	let cursor = 0;
	let fence: { marker: string; length: number } | undefined;
	let inlineTicks = 0;
	while (cursor < markdown.length) {
		if (cursor === 0 || markdown[cursor - 1] === "\n") {
			const end = markdown.indexOf("\n", cursor);
			const lineEnd = end < 0 ? markdown.length : end;
			const line = markdown.slice(cursor, lineEnd);
			const marker =
				/^\s*(?:>\s*)*(?:(?:[-+*]|\d+[.)])\s+)?(`{3,}|~{3,})(.*)$/.exec(line);
			if (fence) {
				if (
					marker &&
					marker[1][0] === fence.marker &&
					marker[1].length >= fence.length &&
					!marker[2].trim()
				)
					fence = undefined;
				cursor = lineEnd + 1;
				continue;
			}
			if (!inlineTicks && marker) {
				fence = { marker: marker[1][0], length: marker[1].length };
				cursor = lineEnd + 1;
				continue;
			}
			if (/^(?: {4}|\t)/.test(line)) {
				cursor = lineEnd + 1;
				continue;
			}
		}
		if (markdown[cursor] === "\\" && !inlineTicks) {
			cursor += 2;
			continue;
		}
		if (markdown[cursor] === "`") {
			let end = cursor + 1;
			while (markdown[end] === "`") end++;
			const count = end - cursor;
			if (!inlineTicks) inlineTicks = count;
			else if (inlineTicks === count) inlineTicks = 0;
			cursor = end;
			continue;
		}
		if (inlineTicks || markdown[cursor] !== "<") {
			cursor++;
			continue;
		}
		if (markdown.startsWith("<!--", cursor)) {
			const end = markdown.indexOf("-->", cursor + 4);
			if (end < 0) return unchanged;
			cursor = end + 3;
			continue;
		}
		if (/^<img(?=\s|\/|>|$)/.test(markdown.slice(cursor))) {
			const tail = readImageTail(markdown, cursor);
			if (!tail) return unchanged;
			if ("end" in tail) {
				cursor = tail.end;
				continue;
			}
			return {
				markdown: `${markdown.slice(0, cursor)}<img ${tail.attributes.join(" ")} streamingPending="true" />`,
				pending: true,
			};
		}
		// Do not discover an image inside another tag's quoted attribute.
		if (/^<\/?[A-Za-z!?]/.test(markdown.slice(cursor))) {
			let quote = "";
			while (++cursor < markdown.length) {
				const char = markdown[cursor];
				if (quote) {
					if (char === quote) quote = "";
				} else if (char === '"' || char === "'") quote = char;
				else if (char === ">") break;
			}
		}
		cursor++;
	}
	return unchanged;
}
