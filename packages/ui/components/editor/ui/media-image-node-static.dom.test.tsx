import { afterAll, afterEach, describe, expect, test } from "bun:test";
import { createElement } from "react";
import { installStageDom } from "../../interfaces/form-workbench/stage/dom-setup";
import { normalizeStreamingEditorialImage } from "../streaming-editorial-image";

const dom = installStageDom();
const { TextEditor } = await import("../../ui/text-editor");
const { IsStreamingMarkdownContext } = await import("../renderer-presentation");

afterEach(dom.cleanup);
afterAll(dom.restore);

function renderImage(source: string, streaming = true) {
	const { markdown } = streaming
		? normalizeStreamingEditorialImage(source)
		: { markdown: source };
	return createElement(
		IsStreamingMarkdownContext.Provider,
		{ value: streaming },
		createElement(TextEditor, {
			initialContent: markdown,
			isMarkdown: true,
			presentation: "chat",
		}),
	);
}

describe("editorial image streaming display", () => {
	test("replaces an unfinished tag with a placeholder, then a safe accessible image", async () => {
		const view = await dom.render(
			renderImage('<img src="/sedan.svg" alt="Ant'),
		);
		expect(
			document.querySelector('[data-slot="editorial-image-pending"]'),
		).not.toBeNull();
		expect(document.querySelector("img")).toBeNull();
		expect(document.querySelector("figure")?.getAttribute("aria-busy")).toBe(
			"true",
		);
		expect(document.body.textContent).not.toContain("<img");
		expect(document.body.textContent).not.toContain("Ant");

		await view.rerender(
			renderImage('<img src="/sedan.svg" alt="Anthracite sedan" caption="View'),
		);
		expect(
			document.querySelector('[data-slot="editorial-image-pending"]'),
		).toBeNull();
		expect(document.querySelector("img")?.getAttribute("src")).toBe(
			"/sedan.svg",
		);
		expect(document.querySelector("img")?.getAttribute("alt")).toBe(
			"Anthracite sedan",
		);
		expect(document.body.textContent).not.toContain("View");

		await view.rerender(
			renderImage(
				'<img src="/sedan.svg" alt="Anthracite sedan" caption="View from the front" />',
				false,
			),
		);
		expect(document.querySelector("figure")?.hasAttribute("aria-busy")).toBe(
			false,
		);
		expect(document.querySelector("figcaption")?.textContent).toBe(
			"View from the front",
		);
		expect(document.querySelector("img")?.getAttribute("src")).toBe(
			"/sedan.svg",
		);
	});

	test("unsafe completed source stays a placeholder without exposing raw markup", async () => {
		await dom.render(
			renderImage('<img src="java&#x09;script:alert(1)" alt="A" caption="Half'),
		);
		expect(
			document.querySelector('[data-slot="editorial-image-pending"]'),
		).not.toBeNull();
		expect(document.querySelector("img")).toBeNull();
		expect(document.body.textContent).not.toContain("script:");
	});

	test("quoted numeric alt text is retained as text", async () => {
		await dom.render(renderImage('<img src="/a.svg" alt="123" caption="Half'));
		expect(document.querySelector("img")?.getAttribute("alt")).toBe("123");
	});
});
