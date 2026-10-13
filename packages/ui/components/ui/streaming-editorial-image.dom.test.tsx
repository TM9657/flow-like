import { afterAll, afterEach, describe, expect, test } from "bun:test";
import { createElement } from "react";
import { installStageDom } from "../interfaces/form-workbench/stage/dom-setup";

const dom = installStageDom();
const { StreamingTextEditor } = await import("./streaming-text-editor");
const { TextEditor, peekSettledParse } = await import("./text-editor");

afterEach(dom.cleanup);
afterAll(dom.restore);

describe("editorial images while streaming", () => {
	test("keeps incomplete attributes out of prose and reveals a usable image progressively", async () => {
		const content =
			'<img src="/example.png" alt="An example" caption="A caption" />';
		const view = await dom.render(
			createElement(StreamingTextEditor, {
				content: "<img",
				presentation: "chat",
			}),
		);
		for (let end = 4; end < content.length; end++) {
			const prefix = content.slice(0, end);
			await view.rerender(
				createElement(StreamingTextEditor, {
					content: prefix,
					presentation: "chat",
				}),
			);
			expect(document.body.textContent).not.toContain("<img");
			expect(document.querySelector("figure")?.getAttribute("aria-busy")).toBe(
				"true",
			);
			expect(peekSettledParse(prefix)).toBeUndefined();
			if (prefix.includes('alt="An example"')) {
				expect(document.querySelector("img")?.getAttribute("src")).toBe(
					"/example.png",
				);
				expect(document.querySelector("img")?.getAttribute("alt")).toBe(
					"An example",
				);
			} else {
				expect(
					document.querySelector("[data-slot=editorial-image-pending]"),
				).not.toBeNull();
				expect(document.querySelector("img")).toBeNull();
			}
		}
		await view.rerender(
			createElement(StreamingTextEditor, { content, presentation: "chat" }),
		);
		expect(document.querySelector("figure")?.hasAttribute("aria-busy")).toBe(
			false,
		);
		expect(document.querySelector("figcaption")?.textContent).toBe("A caption");
		expect(
			document.querySelector("[data-slot=editorial-image-pending]"),
		).toBeNull();
		expect(peekSettledParse(content)).toBeDefined();
		await view.rerender(
			createElement(TextEditor, {
				initialContent: content,
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(document.querySelector("img")?.getAttribute("src")).toBe(
			"/example.png",
		);
		expect(document.querySelector("figcaption")?.textContent).toBe("A caption");
	});

	test("settling an unfinished tag parses the original source without caching the placeholder", async () => {
		const content =
			'An interrupted example.\n\n<img src="/stopped.png" alt="Unfinished';
		const view = await dom.render(
			createElement(StreamingTextEditor, { content, presentation: "chat" }),
		);
		expect(
			document.querySelector("[data-slot=editorial-image-pending]"),
		).not.toBeNull();
		expect(peekSettledParse(content)).toBeUndefined();
		await view.rerender(
			createElement(TextEditor, {
				initialContent: content,
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(
			document.querySelector("[data-slot=editorial-image-pending]"),
		).toBeNull();
		expect(document.querySelector("img")).toBeNull();
		expect(document.body.textContent).toContain('<img src="/stopped.png"');
	});
});
