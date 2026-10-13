import { afterAll, afterEach, describe, expect, test } from "bun:test";
import { createElement } from "react";
import type { IBoard } from "../../lib";
import { installStageDom } from "../interfaces/form-workbench/stage/dom-setup";
import { click } from "../settings/devices/testing/dom-harness";

const dom = installStageDom();
const { MessageContent } = await import("./MessageContent");

afterEach(dom.cleanup);
afterAll(dom.restore);

function required(selector: string): Element {
	const element = document.querySelector(selector);
	if (!element) throw new Error(`Missing ${selector}`);
	return element;
}

describe("Flowpilot streamed message rendering", () => {
	test("live content uses compact Markdown and reveals completed rich items before JSON closes", async () => {
		const prefix = 'A **preview**.\n\n```gallery\n{"items":[';
		const view = await dom.render(
			createElement(MessageContent, { content: prefix, isStreaming: true }),
		);
		expect(
			required('[data-fl-renderer="chat-compact"] strong').textContent,
		).toBe("preview");
		expect(required('[data-slot="rich-media-skeleton"]')).toBeTruthy();
		expect(document.querySelector("pre")).toBeNull();
		const firstItem = '{"src":"/one.svg","alt":"First image"}';
		await view.rerender(
			createElement(MessageContent, {
				content: `${prefix}${firstItem},{"src":"/unfinished`,
				isStreaming: true,
			}),
		);
		expect(required('[data-slot="gallery-grid"] img').getAttribute("alt")).toBe(
			"First image",
		);
		expect(document.querySelector("pre")).toBeNull();
		await view.rerender(
			createElement(MessageContent, {
				content: [`${prefix}${firstItem}]}`, "```"].join("\n"),
				isStreaming: false,
			}),
		);
		expect(
			document.querySelectorAll('[data-slot="gallery-grid"] img').length,
		).toBe(1);
		expect(
			document.querySelector('[data-slot="rich-media-skeleton"]'),
		).toBeNull();
	});

	for (const thinking of ["<think>Working", "<think>Working</think>"]) {
		test(`focus links remain actionable around ${thinking.includes("</") ? "completed" : "open"} thinking`, async () => {
			const focused: string[] = [];
			const board = {
				nodes: { n1: { friendly_name: "Load data" } },
			} as unknown as IBoard;
			await dom.render(
				createElement(MessageContent, {
					content: `<focus_node>n1</focus_node>\n\n${thinking}`,
					isStreaming: true,
					board,
					onFocusNode: (id) => focused.push(id),
				}),
			);
			const link = required('[data-focus-node-id="n1"]');
			expect(link.textContent).toContain("Load data");
			await click(link);
			expect(focused).toEqual(["n1"]);
			expect(document.body.textContent).not.toContain("<focus_node>");
		});
	}

	test("streaming state reaches charts while explicit plain text stays available", async () => {
		const content = '```plotly\n{"data":[';
		const view = await dom.render(
			createElement(MessageContent, { content, isStreaming: true }),
		);
		expect(required("[data-fl-chart-pending]")).toBeTruthy();
		expect(document.querySelector("pre")).toBeNull();
		await view.rerender(
			createElement(MessageContent, { content, isStreaming: false }),
		);
		expect(required("pre").textContent).toContain('"data":[');
		await view.rerender(
			createElement(MessageContent, {
				content: "**Plain**",
				isStreaming: true,
				enableMarkdown: false,
			}),
		);
		expect(document.querySelector("strong")).toBeNull();
		expect(document.body.textContent).toContain("**Plain**");
	});
});
