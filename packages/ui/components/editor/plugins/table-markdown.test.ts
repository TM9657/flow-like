import { describe, expect, test } from "bun:test";
import { MarkdownPlugin } from "@platejs/markdown";
import {
	type TTableElement,
	type TTableRowElement,
	createSlateEditor,
} from "platejs";
import {
	EMPTY_STREAMING_STATE,
	type StreamingParseState,
	parseStreamingMarkdown,
} from "../../ui/streaming-markdown-blocks";
import { RICH_REMARK_PLUGINS, safeDeserialize } from "../../ui/text-editor";
import { BaseEditorKit } from "../editor-base-kit";

const worker = createSlateEditor({ plugins: BaseEditorKit, nodeId: false });
const parse = (markdown: string) =>
	safeDeserialize(worker, markdown, true, RICH_REMARK_PLUGINS);
const markdown = [
	"| Model | Engine | PS | Notes |",
	"| :--- | :---: | ---: | --- |",
	"| **BMW 750d** | `diesel` | 400 | [Source](https://example.com/specs) |",
	"| Panamera | petrol | 330 | Available |",
].join("\n");

describe("GFM table alignment", () => {
	test("preserves explicit alignment in every row alongside rich content", () => {
		const [table] = parse(markdown) as TTableElement[];
		expect(table.type).toBe("table");
		const rows = table.children as TTableRowElement[];
		for (const row of rows) {
			expect(row.children.map((cell) => cell.align)).toEqual([
				"left",
				"center",
				"right",
				undefined,
			]);
			expect(row.children[3]).not.toHaveProperty("align");
		}
		expect(rows[1].children[0].children).toEqual([
			{ type: "p", children: [{ text: "BMW 750d", bold: true }] },
		]);
		expect(rows[1].children[1].children).toEqual([
			{ type: "p", children: [{ text: "diesel", code: true }] },
		]);
		expect(rows[1].children[3].children).toEqual([
			{
				type: "p",
				children: [
					{
						type: "a",
						url: "https://example.com/specs",
						children: [{ text: "Source" }],
					},
				],
			},
		]);
	});

	test("streaming and settled parsing agree while the table grows", () => {
		const document = `## Comparison\n\n${markdown}\n\nChoose by engine.`;
		let state: StreamingParseState = EMPTY_STREAMING_STATE;
		for (let cut = 1; cut <= document.length; cut++) {
			const prefix = document.slice(0, cut);
			state = parseStreamingMarkdown(worker, prefix, state);
			expect({ prefix, value: state.blocks }).toEqual({
				prefix,
				value: parse(prefix),
			});
		}
	});

	test("Markdown export retains column alignment and cell formatting", () => {
		const value = parse(markdown);
		const serialized = worker.getApi(MarkdownPlugin).markdown.serialize({
			value,
		});
		expect(parse(serialized)).toEqual(value);
	});

	test("unmarked columns keep their existing shape", () => {
		const [table] = parse(
			"| A | B |\n| --- | --- |\n| 1 | 2 |",
		) as TTableElement[];
		for (const row of table.children as TTableRowElement[])
			for (const cell of row.children) expect(cell).not.toHaveProperty("align");
	});
});
