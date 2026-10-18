// Read balanced Rust blocks while ignoring strings and comments. The adapter
// copies upstream items, so schema and dispatcher changes come from the pin.
export function blockEnd(source, start, open = "{", close = "}") {
	if (source[start] !== open) throw new Error(`Expected ${open} at ${start}`);
	let depth = 0;
	for (let i = start; i < source.length; i++) {
		if (source.startsWith("//", i)) {
			i = source.indexOf("\n", i);
			if (i < 0) break;
			continue;
		}
		if (source.startsWith("/*", i)) {
			let nesting = 1;
			i += 2;
			for (; i < source.length && nesting; i++) {
				if (source.startsWith("/*", i)) {
					nesting++;
					i++;
				} else if (source.startsWith("*/", i)) {
					nesting--;
					i++;
				}
			}
			i--;
			continue;
		}
		const raw = source.slice(i).match(/^r(#+)?"/);
		if (raw) {
			const end = source.indexOf(`"${raw[1] || ""}`, i + raw[0].length);
			if (end < 0) break;
			i = end + (raw[1]?.length || 0);
			continue;
		}
		if (
			source[i] === '"' ||
			(source[i] === "'" && /^'(?:\\.|[^'\n\\])'/.test(source.slice(i)))
		) {
			const quote = source[i++];
			for (; i < source.length; i++) {
				if (source[i] === "\\") i++;
				else if (source[i] === quote) break;
			}
			continue;
		}
		if (source[i] === open) depth++;
		if (source[i] === close && --depth === 0) return i + 1;
	}
	throw new Error("Unclosed Rust block; review the upstream MCP adapter.");
}

export function rustFunction(source, name, { attributes = false } = {}) {
	const pattern = new RegExp(
		`(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn ${name}\\b`,
	);
	const found = pattern.exec(source);
	if (!found) throw new Error(`Upstream MCP function ${name} is missing.`);
	let start = found.index;
	if (attributes) {
		const attr = source.lastIndexOf("#[tool(", start);
		if (attr < 0 || source.slice(attr, start).includes("async fn"))
			throw new Error(`Missing tool annotation for ${name}.`);
		start = attr;
	}
	const body = source.indexOf("{", found.index);
	return source.slice(start, blockEnd(source, body));
}

// Native headless work is a different session. Keep each upstream bridge branch.
export function bridgeBranch(source) {
	while (/if let Some\(r\) = self\s*\.headless_op/.test(source)) {
		const start = source.search(/if let Some\(r\) = self\s*\.headless_op/);
		const awaited = source.indexOf(".await", start);
		const body = source.indexOf("{", awaited);
		const end = blockEnd(source, body);
		const after = source.slice(end);
		const alternate = /^\s*else\s*\{/.exec(after);
		if (alternate) {
			const altStart = end + alternate[0].lastIndexOf("{");
			const altEnd = blockEnd(source, altStart);
			source =
				source.slice(0, start) +
				source.slice(altStart, altEnd) +
				source.slice(altEnd);
		} else source = source.slice(0, start) + source.slice(end);
	}
	if (source.includes("headless_op"))
		throw new Error(
			"Upstream headless MCP dispatch changed; review its browser adapter.",
		);
	return source;
}
