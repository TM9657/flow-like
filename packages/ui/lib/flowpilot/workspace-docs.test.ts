import { describe, expect, it } from "vitest";
import {
	FLOWPILOT_DOC_CORPUS_BYTES,
	FLOWPILOT_DOC_PATHS,
	FLOWPILOT_DOC_SECTION_BYTES,
	buildFlowPilotDocsCorpus,
	parseFlowPilotDoc,
} from "../../../../apps/docs/scripts/generate-flowpilot-corpus";
import { loadFlowPilotWorkspaceDocs } from "./workspace-docs";

describe("bundled FlowPilot workspace documentation", () => {
	it("matches the current selected source documents with explicit limited coverage", async () => {
		const bundled = await loadFlowPilotWorkspaceDocs();
		expect(bundled).toEqual(await buildFlowPilotDocsCorpus());
		expect(bundled.coverage.source_paths).toEqual([...FLOWPILOT_DOC_PATHS]);
		expect(bundled.coverage.mode).toBe("bundled_selected_docs");
		expect(bundled.coverage.excluded).toContain(
			"Documentation outside the listed source paths",
		);
		expect(Buffer.byteLength(JSON.stringify(bundled))).toBeLessThanOrEqual(
			FLOWPILOT_DOC_CORPUS_BYTES,
		);
	});

	it("retains exact source provenance and bounded sections for app integration topics", async () => {
		const corpus = await loadFlowPilotWorkspaceDocs();
		expect(new Set(corpus.entries.map((entry) => entry.resource_id)).size).toBe(
			corpus.entries.length,
		);
		for (const entry of corpus.entries) {
			expect(entry.kind).toBe("doc");
			expect(entry.resource_id).toBe(
				`doc:${entry.source_path}#${entry.section_id}`,
			);
			expect(entry.line_start).toBeGreaterThan(0);
			expect(entry.line_end).toBeGreaterThanOrEqual(entry.line_start);
			expect(Buffer.byteLength(entry.content)).toBeLessThanOrEqual(
				FLOWPILOT_DOC_SECTION_BYTES,
			);
			expect(entry.revision).toMatch(/^[a-f0-9]{64}$/);
			expect(entry.content_revision).toMatch(/^[a-f0-9]{64}$/);
		}
		for (const [section, phrase] of [
			["studio/flowscript.md#functions-and-handlers", "function"],
			["dev/a2ui/pages.md#page-event-and-route", "Event"],
			["dev/a2ui/overview.md#actions-and-workflow-updates", "action"],
			["apps/data-studio.md#native-tables", "schema"],
			["apps/runtime-variables.md#where-values-go", "secret"],
		]) {
			const entry = corpus.entries.find((item) =>
				item.resource_id.endsWith(section),
			);
			expect(entry, section).toBeDefined();
			expect(entry?.content.toLowerCase()).toContain(phrase.toLowerCase());
		}
	});

	it("keeps code literal while identifying prose headings and source lines", () => {
		const entries = parseFlowPilotDoc(
			"fixture.md",
			[
				"---",
				"title: Integration",
				"---",
				"**Intro** with [details](/details).",
				"![Screenshot](private-image.png)",
				"## Contract",
				"```python",
				"# This is code, not a section",
				"value = '`literal` **exact**'",
				"```",
				"## Contract",
				"Keep the next contract separate.",
			].join("\n"),
		);
		expect(entries.map((entry) => entry.section_id)).toEqual([
			"introduction",
			"contract",
			"contract-2",
		]);
		expect(entries[0].content).toBe("Intro with details (/details).");
		expect(entries[1].content).toBe(
			"# This is code, not a section\nvalue = '`literal` **exact**'",
		);
		expect(entries[1].line_start).toBe(8);
		expect(entries[1].line_end).toBe(9);
	});

	it("splits long sections without losing Unicode text and gives chunks distinct IDs", () => {
		const text = "🧩".repeat(2_003);
		const entries = parseFlowPilotDoc("fixture.md", `## Long\n${text}`);
		expect(entries.length).toBe(3);
		expect(entries.map((entry) => entry.content).join("")).toBe(text);
		expect(entries.map((entry) => entry.section_id)).toEqual([
			"long--part-1",
			"long--part-2",
			"long--part-3",
		]);
		for (const entry of entries) {
			expect(Buffer.byteLength(entry.content)).toBeLessThanOrEqual(
				FLOWPILOT_DOC_SECTION_BYTES,
			);
			expect(entry.content).not.toContain("\uFFFD");
		}
	});

	it("changes source revisions without changing unaffected section content revisions", () => {
		const original = parseFlowPilotDoc(
			"fixture.md",
			"## Read\nOne\n## Write\nTwo",
		);
		const changed = parseFlowPilotDoc(
			"fixture.md",
			"## Read\nOne\n## Write\nThree",
		);
		expect(changed[0].resource_id).toBe(original[0].resource_id);
		expect(changed[0].revision).not.toBe(original[0].revision);
		expect(changed[0].content_revision).toBe(original[0].content_revision);
		expect(changed[1].content_revision).not.toBe(original[1].content_revision);
	});

	it("fails generation on malformed sources instead of publishing partial coverage", () => {
		expect(() =>
			parseFlowPilotDoc("fixture.md", "---\ntitle: Incomplete"),
		).toThrow("Unclosed frontmatter");
		expect(() => parseFlowPilotDoc("fixture.md", "```ts\nvalue()")).toThrow(
			"Unclosed code fence",
		);
	});

	it("keeps resource IDs unique when headings collide with generated suffixes", () => {
		const entries = parseFlowPilotDoc(
			"fixture.md",
			"Intro\n## Introduction\nA\n## Contract\nB\n## Contract\nC\n## Contract 2\nD",
		);
		expect(entries.map((entry) => entry.section_id)).toEqual([
			"introduction",
			"introduction-2",
			"contract",
			"contract-2",
			"contract-2-2",
		]);
	});
});
