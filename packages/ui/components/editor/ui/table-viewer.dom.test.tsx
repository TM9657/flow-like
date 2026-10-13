import { afterAll, afterEach, describe, expect, test } from "bun:test";
import { act, createElement } from "react";
import { installStageDom } from "../../interfaces/form-workbench/stage/dom-setup";
import {
	click,
	keyDown,
	settle,
	typeInto,
} from "../../settings/devices/testing/dom-harness";

const dom = installStageDom();
const { TableViewer } = await import("./table-viewer");
const { createSlateEditor } = await import("platejs");
const { PlateStatic } = await import("platejs/static");
const { BaseEditorKit } = await import("../editor-base-kit");
const { RendererPresentationContext } = await import(
	"../renderer-presentation"
);
const {
	TextEditor,
	getStaticParseWorker,
	safeDeserialize,
	RICH_REMARK_PLUGINS,
} = await import("../../ui/text-editor");
const { indexEditorPaths } = await import("../../ui/lazy-plate-static");
const { StreamingTextEditor } = await import("../../ui/streaming-text-editor");

afterEach(dom.cleanup);
afterAll(dom.restore);

const DATA = [
	["Model", "Power", "Notes"],
	[
		"BMW 750d",
		"400",
		"A long description that should remain readable without clicking an individual cell.",
	],
	["Panamera", "422", "Diesel"],
	["Mercedes", "340", "Diesel"],
];

function query<T extends Element = HTMLElement>(selector: string): T {
	const result = document.querySelector<T>(selector);
	if (!result) throw new Error(`Missing ${selector}`);
	return result;
}

function button(label: string): HTMLButtonElement {
	const result = Array.from(document.querySelectorAll("button")).find((node) =>
		(
			node.getAttribute("aria-label") ??
			node.textContent?.trim() ??
			""
		).startsWith(label),
	);
	if (!result) throw new Error(`Missing button ${label}`);
	return result;
}

const bodyRows = () => Array.from(query("table").querySelectorAll("tbody tr"));
const modelOrder = () => bodyRows().map((row) => row.children[0]?.textContent);

async function mountViewer() {
	return dom.render(
		createElement(TableViewer, {
			data: DATA,
			appearance: "chat",
			richCells: DATA.map((row, index) =>
				row.map((text, column) => ({
					content:
						column === 0 && index > 0
							? createElement(
									"a",
									{ href: `https://example.com/${index}` },
									createElement("strong", null, text),
								)
							: text,
				})),
			),
		}),
	);
}

describe("chat table interactions", () => {
	test("starts wrapped with all table actions available and keeps links out of buttons", async () => {
		await mountViewer();
		expect(
			query("[data-fl-table-appearance]").getAttribute("data-table-wrap"),
		).toBe("true");
		expect(query("[data-slot=table-toolbar]").className).not.toContain(
			"opacity-0",
		);
		for (const name of [
			"Filter",
			"Columns",
			"Wrap",
			"Copy as Markdown",
			"Download CSV",
			"Sort by Power",
			"Reorder Model column",
		])
			expect(button(name)).toBeTruthy();
		expect(query("tbody a").closest("button")).toBeNull();
		await click(button("Wrap"));
		expect(button("Wrap").getAttribute("aria-pressed")).toBe("false");
		expect(query("tbody [data-slot=table-cell-content]").className).toContain(
			"whitespace-nowrap",
		);
		expect(query("tbody a").getAttribute("href")).toBe("https://example.com/1");
		await click(button("Wrap"));
		expect(button("Wrap").getAttribute("aria-pressed")).toBe("true");
	});

	test("sorting and searching preserve the rich cell's source row", async () => {
		await mountViewer();
		await click(button("Sort by Power"));
		expect(modelOrder()).toEqual(["Mercedes", "BMW 750d", "Panamera"]);
		expect(query("tbody a").getAttribute("href")).toBe("https://example.com/3");
		expect(query("thead th:nth-child(2)").getAttribute("aria-sort")).toBe(
			"ascending",
		);
		await click(button("Sort by Power"));
		expect(modelOrder()).toEqual(["Panamera", "BMW 750d", "Mercedes"]);
		await click(button("Sort by Power"));
		expect(modelOrder()).toEqual(["BMW 750d", "Panamera", "Mercedes"]);
		await typeInto(query("input[aria-label='Search table']"), "mercedes");
		expect(modelOrder()).toEqual(["Mercedes"]);
		expect(query("tbody strong").textContent).toBe("Mercedes");
		await click(button("Clear"));
		expect(bodyRows()).toHaveLength(3);
	});

	test("column filtering and visibility affect copied data", async () => {
		await mountViewer();
		await click(button("Filter"));
		await typeInto(query("input[aria-label='Filter Notes']"), "Diesel");
		expect(modelOrder()).toEqual(["Panamera", "Mercedes"]);
		await click(button("Filter"));
		await click(button("Columns"));
		await click(query("#col-visibility-2"));
		await click(button("Columns"));
		expect(query("table thead tr").children).toHaveLength(2);
		await click(button("Copy as Markdown"));
		expect(dom.clipboard.at(-1)).toBe(
			"| Model | Power |\n| --- | --- |\n| Panamera | 422 |\n| Mercedes | 340 |",
		);
		await click(button("Columns"));
		await click(button("Reset"));
		await click(button("Columns"));
		expect(query("table thead tr").children).toHaveLength(3);
	});

	test("downloads the current rows and columns as CSV", async () => {
		await mountViewer();
		await typeInto(query("input[aria-label='Search table']"), "BMW");
		let downloaded: HTMLAnchorElement | undefined;
		const original = dom.window.HTMLAnchorElement.prototype.click;
		dom.window.HTMLAnchorElement.prototype.click = function () {
			downloaded = this as unknown as HTMLAnchorElement;
		};
		try {
			await click(button("Download CSV"));
			expect(downloaded?.download).toBe("table.csv");
			expect(decodeURIComponent(downloaded?.href ?? "")).toContain(
				"Model,Power,Notes\nBMW 750d,400,",
			);
			expect(decodeURIComponent(downloaded?.href ?? "")).not.toContain(
				"Panamera",
			);
		} finally {
			dom.window.HTMLAnchorElement.prototype.click = original;
		}
	});
});

test("columns can be reordered from a keyboard and the rich cells follow", async () => {
	await mountViewer();
	for (const [index, cell] of Array.from(
		document.querySelectorAll("thead th"),
	).entries()) {
		cell.getBoundingClientRect = () => ({
			x: index * 150,
			y: 0,
			width: 150,
			height: 40,
			top: 0,
			bottom: 40,
			left: index * 150,
			right: (index + 1) * 150,
			toJSON: () => ({}),
		});
	}
	const handle = button("Reorder Model column");
	await act(async () => handle.focus());
	await keyDown(handle, " ", { code: "Space" });
	await settle();
	await keyDown(handle, "ArrowRight", { code: "ArrowRight" });
	await keyDown(handle, " ", { code: "Space" });
	expect(query("tbody tr").children[0]?.textContent).toBe("400");
	expect(
		query("tbody tr").children[1]?.querySelector("a")?.getAttribute("href"),
	).toBe("https://example.com/1");
	await click(button("Copy as Markdown"));
	expect(dom.clipboard.at(-1)).toStartWith("| Power | Model | Notes |");
});

test("the static chat adapter renders original marks, code, links and alignment once", async () => {
	const value = safeDeserialize(
		getStaticParseWorker(BaseEditorKit),
		"| Model | Source | Power |\n| --- | --- | --- |\n| **BMW** | [Details](https://example.com/bmw) | `400` |",
		true,
		RICH_REMARK_PLUGINS,
	);
	const cell = (value[0] as { children: { children: { align?: string }[] }[] })
		.children[0].children[2];
	cell.align = "right";
	const editor = createSlateEditor({
		plugins: BaseEditorKit,
		value,
		nodeId: false,
	});
	indexEditorPaths(editor);
	await dom.render(
		createElement(
			RendererPresentationContext.Provider,
			{ value: "chat" },
			createElement(PlateStatic, { editor }),
		),
	);
	const table = query("table");
	const links = table.querySelectorAll('a[href="https://example.com/bmw"]');
	expect(links).toHaveLength(1);
	expect(table.querySelector("tbody strong")?.textContent).toBe("BMW");
	expect(table.querySelector("tbody code")?.textContent).toBe("400");
	expect(query<HTMLElement>("tbody td:nth-child(3)").style.textAlign).toBe(
		"right",
	);
	expect(document.querySelectorAll("table")).toHaveLength(1);
});

test("streaming and settled chat tables preserve the same rich content and alignment", async () => {
	const content =
		"| Model | Source | Power |\n| :--- | :---: | ---: |\n| **BMW** | [Details](https://example.com/bmw) | `400` |";
	const view = await dom.render(
		createElement(StreamingTextEditor, { content, presentation: "chat" }),
	);
	const inspect = () => ({
		appearance: query("[data-fl-table-appearance]").getAttribute(
			"data-fl-table-appearance",
		),
		wrap: query("[data-fl-table-appearance]").getAttribute("data-table-wrap"),
		bold: query("tbody strong").textContent,
		code: query("tbody code").textContent,
		href: query("tbody a").getAttribute("href"),
		align: Array.from(document.querySelectorAll<HTMLElement>("tbody td")).map(
			(cell) => cell.style.textAlign,
		),
		tables: document.querySelectorAll("table").length,
	});
	const streaming = inspect();
	await view.rerender(
		createElement(TextEditor, {
			initialContent: content,
			isMarkdown: true,
			presentation: "chat",
		}),
	);
	expect(inspect()).toEqual(streaming);
	expect(streaming).toEqual({
		appearance: "chat",
		wrap: "true",
		bold: "BMW",
		code: "400",
		href: "https://example.com/bmw",
		align: ["left", "center", "right"],
		tables: 1,
	});
});

test("the default viewer retains its compact grid controls and cell expansion", async () => {
	await dom.render(createElement(TableViewer, { data: DATA }));
	expect(document.querySelector("[data-fl-table-appearance]")).toBeNull();
	expect(query(".group\\/table > div").className).toContain("opacity-0");
	expect(query(".overflow-auto").className).toContain("max-h-[400px]");
	const cell = query<HTMLButtonElement>("tbody td button");
	expect(cell.getAttribute("aria-expanded")).toBe("false");
	await click(cell);
	expect(cell.getAttribute("aria-expanded")).toBe("true");
});

test("large chat tables expand without a nested vertical scroller and export all matching rows", async () => {
	const data = [
		["Name", "Value"],
		...Array.from({ length: 15 }, (_, index) => [
			`Item ${index + 1}`,
			String(index + 1),
		]),
	];
	await dom.render(createElement(TableViewer, { data, appearance: "chat" }));
	expect(bodyRows()).toHaveLength(10);
	expect(query("[data-slot=table-scroll]").className).not.toContain("max-h-");
	expect(button("Show more (15 rows)").getAttribute("aria-expanded")).toBe(
		"false",
	);
	await click(button("Copy as Markdown"));
	expect(dom.clipboard.at(-1)).toContain("| Item 15 | 15 |");
	await click(button("Show more (15 rows)"));
	expect(bodyRows()).toHaveLength(15);
	await click(button("Show less"));
	expect(bodyRows()).toHaveLength(10);
	await typeInto(query("input[aria-label='Search table']"), "Item 15");
	expect(bodyRows()).toHaveLength(1);
	expect(query("tbody tr").textContent).toBe("Item 1515");
	expect(
		document.querySelector("button[aria-expanded='false']")?.textContent,
	).not.toContain("Show more");
});
