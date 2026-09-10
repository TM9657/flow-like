import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type {
	FlowPilotWorkspaceDocEntry,
	FlowPilotWorkspaceDocsCorpus,
} from "../../../packages/ui/lib/flowpilot/workspace-docs";

export const FLOWPILOT_DOC_PATHS = [
	"apps/docs/src/content/docs/studio/flowscript.md",
	"apps/docs/src/content/docs/studio/variables.md",
	"apps/docs/src/content/docs/apps/events.md",
	"apps/docs/src/content/docs/apps/runtime-variables.md",
	"apps/docs/src/content/docs/apps/data-studio.md",
	"apps/docs/src/content/docs/dev/a2ui/overview.md",
	"apps/docs/src/content/docs/dev/a2ui/pages.md",
	"apps/docs/src/content/docs/dev/a2ui/routes.md",
	"apps/docs/src/content/docs/dev/a2ui/widgets.md",
] as const;

export const FLOWPILOT_DOC_SECTION_BYTES = 4_000;
export const FLOWPILOT_DOC_CORPUS_BYTES = 160_000;
export const FLOWPILOT_DOC_OUTPUT_PATH =
	"packages/ui/lib/flowpilot/generated/workspace-docs.json";
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

function revision(value: string): string {
	return createHash("sha256").update(value).digest("hex");
}

function slug(value: string): string {
	return (
		value
			.toLowerCase()
			.replace(/[^\p{L}\p{N}\s-]/gu, "")
			.trim()
			.replace(/[\s-]+/g, "-") || "section"
	);
}

function prose(value: string): string {
	return value
		.replace(/!\[[^\]]*\]\([^)]*\)/g, "")
		.replace(/\[([^\]]+)\]\(([^)]+)\)/g, "$1 ($2)")
		.replace(/\*\*([^*]+)\*\*/g, "$1")
		.replace(/`([^`]+)`/g, "$1");
}

interface SourceLine {
	text: string;
	line: number;
}

function boundedLines(lines: SourceLine[]): SourceLine[][] {
	const chunks: SourceLine[][] = [];
	let chunk: SourceLine[] = [];
	let size = 0;
	for (const line of lines) {
		// Split unusually long source lines on Unicode code points, never bytes.
		let remainder = line.text;
		while (Buffer.byteLength(remainder) > FLOWPILOT_DOC_SECTION_BYTES) {
			let prefix = "";
			let bytes = 0;
			for (const character of remainder) {
				const length = Buffer.byteLength(character);
				if (bytes + length > FLOWPILOT_DOC_SECTION_BYTES) break;
				prefix += character;
				bytes += length;
			}
			if (chunk.length > 0) chunks.push(chunk);
			chunks.push([{ text: prefix, line: line.line }]);
			chunk = [];
			size = 0;
			remainder = remainder.slice(prefix.length);
		}
		const length = Buffer.byteLength(remainder);
		if (chunk.length > 0 && size + 1 + length > FLOWPILOT_DOC_SECTION_BYTES) {
			chunks.push(chunk);
			chunk = [];
			size = 0;
		}
		size += length + (chunk.length > 0 ? 1 : 0);
		chunk.push({ text: remainder, line: line.line });
	}
	if (chunk.length > 0) chunks.push(chunk);
	return chunks;
}

export function parseFlowPilotDoc(
	sourcePath: string,
	source: string,
): FlowPilotWorkspaceDocEntry[] {
	const sourceRevision = revision(source);
	const lines = source.replace(/\r\n/g, "\n").split("\n");
	let firstLine = 0;
	let title = sourcePath.split("/").at(-1)?.replace(/\.md$/, "") ?? sourcePath;
	if (lines[0] === "---") {
		const end = lines.indexOf("---", 1);
		if (end < 0) throw new Error(`Unclosed frontmatter: ${sourcePath}`);
		for (const line of lines.slice(1, end)) {
			const titleMatch = /^title:\s*(.+)$/.exec(line);
			if (titleMatch) title = titleMatch[1].replace(/^(["'])(.*)\1$/, "$2");
		}
		firstLine = end + 1;
	}

	const entries: FlowPilotWorkspaceDocEntry[] = [];
	const headings: string[] = [];
	const ids = new Set(["introduction"]);
	let sectionId = "introduction";
	let sectionTitle = title;
	let section: SourceLine[] = [];
	let fence: { character: string; length: number } | undefined;

	const flush = () => {
		const chunks = boundedLines(section).filter((chunk) =>
			chunk.some((line) => line.text.trim().length > 0),
		);
		for (const [index, chunk] of chunks.entries()) {
			const content = chunk
				.map((line) => line.text)
				.join("\n")
				.trim();
			const id =
				chunks.length > 1 ? `${sectionId}--part-${index + 1}` : sectionId;
			entries.push({
				kind: "doc",
				resource_id: `doc:${sourcePath}#${id}`,
				title:
					chunks.length > 1
						? `${sectionTitle} (part ${index + 1})`
						: sectionTitle,
				source_path: sourcePath,
				section_id: id,
				revision: sourceRevision,
				content_revision: revision(content),
				content,
				line_start: chunk[0].line,
				line_end: chunk[chunk.length - 1].line,
			});
		}
		section = [];
	};

	for (let index = firstLine; index < lines.length; index++) {
		const line = lines[index];
		const fenceMatch = /^\s*(`{3,}|~{3,})(.*)$/.exec(line);
		if (fenceMatch) {
			if (!fence) {
				fence = { character: fenceMatch[1][0], length: fenceMatch[1].length };
				continue;
			}
			if (
				fenceMatch[1][0] === fence.character &&
				fenceMatch[1].length >= fence.length &&
				!fenceMatch[2].trim()
			) {
				fence = undefined;
				continue;
			}
		}
		const heading = !fence ? /^(#{1,6})\s+(.+?)\s*#*\s*$/.exec(line) : null;
		if (heading) {
			flush();
			const label = prose(heading[2]);
			headings.length = heading[1].length - 1;
			headings[heading[1].length - 1] = label;
			sectionTitle = [title, ...headings.filter(Boolean)].join(" > ");
			const base = slug(label);
			sectionId = base;
			let count = 2;
			while (ids.has(sectionId)) sectionId = `${base}-${count++}`;
			ids.add(sectionId);
			continue;
		}
		section.push({ text: fence ? line : prose(line), line: index + 1 });
	}
	if (fence) throw new Error(`Unclosed code fence: ${sourcePath}`);
	flush();
	return entries;
}

export async function buildFlowPilotDocsCorpus(
	root = ROOT,
): Promise<FlowPilotWorkspaceDocsCorpus> {
	const sources = await Promise.all(
		FLOWPILOT_DOC_PATHS.map(async (sourcePath) => ({
			sourcePath,
			content: await readFile(resolve(root, sourcePath), "utf8"),
		})),
	);
	const entries = sources.flatMap(({ sourcePath, content }) =>
		parseFlowPilotDoc(sourcePath, content),
	);
	const corpus: FlowPilotWorkspaceDocsCorpus = {
		schema_version: 1,
		revision: revision(JSON.stringify(entries)),
		coverage: {
			mode: "bundled_selected_docs",
			complete: true,
			source_paths: [...FLOWPILOT_DOC_PATHS],
			excluded: [
				"Documentation outside the listed source paths",
				"Images and frontmatter other than page titles",
				"Node reference and exact live catalog declarations",
				"Repository implementation files and runtime app contents",
			],
		},
		entries,
	};
	if (Buffer.byteLength(JSON.stringify(corpus)) > FLOWPILOT_DOC_CORPUS_BYTES) {
		throw new Error("FlowPilot documentation corpus exceeds its bundle budget");
	}
	return corpus;
}

if (
	process.argv[1] &&
	resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
	const output = resolve(ROOT, FLOWPILOT_DOC_OUTPUT_PATH);
	const serialized = `${JSON.stringify(await buildFlowPilotDocsCorpus())}\n`;
	if (process.argv.includes("--check")) {
		if ((await readFile(output, "utf8")) !== serialized) {
			throw new Error("FlowPilot documentation corpus is stale; regenerate it");
		}
	} else {
		await mkdir(dirname(output), { recursive: true });
		await writeFile(output, serialized, "utf8");
	}
}
