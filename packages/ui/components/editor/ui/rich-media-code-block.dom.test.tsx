import { afterAll, afterEach, describe, expect, test } from "bun:test";
import { createElement } from "react";
import { installStageDom } from "../../interfaces/form-workbench/stage/dom-setup";
import { click, fire } from "../../settings/devices/testing/dom-harness";

const dom = installStageDom();
const { TextEditor } = await import("../../ui/text-editor");
const { StreamingTextEditor } = await import("../../ui/streaming-text-editor");
const { ChartCodeBlock } = await import("./chart-code-block");
const { RendererPresentationContext } = await import(
	"../renderer-presentation"
);

afterEach(dom.cleanup);
afterAll(dom.restore);

const examples = {
	gallery: {
		title: "Illustrative gallery",
		items: [
			{
				src: "/assets/a.svg",
				alt: "Model A",
				caption: "A <script> and `code` remain text",
				credit: "Illustration",
			},
			{
				src: "javascript:alert(1)",
				alt: "Unavailable image",
				href: "javascript:alert(1)",
			},
		],
	},
	recommendations: {
		title: "Ranked examples",
		items: [
			{
				title: "Model A",
				badge: "Best fit",
				image: { src: "/assets/a.svg", alt: "Model A" },
				specs: [{ label: "Power", value: "400 PS" }],
				description: "Illustrative description",
				href: "https://example.com/a",
			},
			{ title: "Model B", subtitle: "Alternative" },
		],
	},
	comparison: {
		title: "Illustrative ratings",
		description: "Subjective scale of 0 to 5",
		max: 5,
		items: [
			{ label: "A", value: 4.5, valueLabel: "Very good" },
			{ label: "B", value: 3, detail: "More reserved" },
		],
	},
} as const;

function required<T extends Element = HTMLElement>(selector: string): T {
	const element = document.querySelector<T>(selector);
	if (!element) throw new Error(`Missing ${selector}`);
	return element;
}

const fence = (language: string, content: string) =>
	`\`\`\`${language}\n${content}\n\`\`\``;

describe("rich fences in chat", () => {
	for (const [language, value] of Object.entries(examples)) {
		test(`${language} keeps its preview visible at every streamed character`, async () => {
			const header = `\`\`\`${language}\n`;
			const content = fence(language, JSON.stringify(value));
			const view = await dom.render(
				createElement(StreamingTextEditor, {
					content: header,
					presentation: "chat",
				}),
			);
			const itemSelector =
				language === "gallery"
					? "[data-slot=gallery-grid] > figure"
					: language === "recommendations"
						? "[data-slot=recommendation-list] > li"
						: "[role=meter]";
			let previousCount = 0;
			for (let end = header.length; end <= content.length; end++) {
				await view.rerender(
					createElement(StreamingTextEditor, {
						content: content.slice(0, end),
						presentation: "chat",
					}),
				);
				expect(required("[data-fl-rich-media]")).toBeTruthy();
				expect(document.querySelector("pre")).toBeNull();
				const count = document.querySelectorAll(itemSelector).length;
				expect(
					count,
					`preview lost items at character ${end}`,
				).toBeGreaterThanOrEqual(previousCount);
				previousCount = count;
			}
			expect(previousCount).toBe(value.items.length);
			const tracking = required("[data-fl-rich-media]")
				.nextElementSibling as HTMLElement;
			expect(tracking.style.display).toBe("none");
			expect(tracking.getAttribute("aria-hidden")).toBe("true");
		});

		test(`${language} renders completed fields while streaming and matches the settled renderer`, async () => {
			const json = JSON.stringify(value);
			const incomplete = `\`\`\`${language}\n${json.slice(0, -1)}`;
			const view = await dom.render(
				createElement(StreamingTextEditor, {
					content: incomplete,
					presentation: "chat",
				}),
			);
			expect(
				document.querySelector("[data-slot=rich-media-source]"),
			).toBeNull();
			expect(required("[data-fl-rich-media]").getAttribute("aria-busy")).toBe(
				"true",
			);
			expect(
				document.querySelector("[data-slot=rich-media-skeleton]"),
			).toBeNull();
			const content = fence(language, json);
			await view.rerender(
				createElement(StreamingTextEditor, { content, presentation: "chat" }),
			);
			const rendered = required("[data-fl-rich-media]").outerHTML;
			await view.rerender(
				createElement(TextEditor, {
					initialContent: content,
					isMarkdown: true,
					presentation: "chat",
				}),
			);
			expect(required("[data-fl-rich-media]").outerHTML).toBe(rendered);
			expect(
				required("[data-fl-rich-media]").getAttribute(
					"data-fl-rich-media-appearance",
				),
			).toBe("chat");
			expect(document.querySelector("script")).toBeNull();
			expect(
				document.querySelector('[href^="javascript:"], [src^="javascript:"]'),
			).toBeNull();
			await click(required("[data-slot=rich-media-source-toggle]"));
			expect(required("[data-slot=rich-media-source]").textContent).toBe(json);
			await click(required("[data-slot=rich-media-source-toggle]"));
			expect(
				document.querySelector("[data-slot=rich-media-source]"),
			).toBeNull();
		});
	}

	test("gallery starts with placeholders and adds images as their required fields arrive", async () => {
		const base = '```gallery\n{"title":"Gallery","items":[';
		const view = await dom.render(
			createElement(StreamingTextEditor, {
				content: base,
				presentation: "chat",
			}),
		);
		expect(
			required("[data-slot=rich-media-skeleton]").getAttribute("data-kind"),
		).toBe("gallery");
		expect(document.querySelector("[data-slot=rich-media-source]")).toBeNull();
		const first = `${base}{"src":"/a.svg","alt":"A"},{"src":"/b.svg","alt":"B`;
		await view.rerender(
			createElement(StreamingTextEditor, {
				content: first,
				presentation: "chat",
			}),
		);
		expect(
			document.querySelectorAll("[data-slot=gallery-grid] img"),
		).toHaveLength(1);
		expect(required("[data-slot=gallery-grid] img").getAttribute("alt")).toBe(
			"A",
		);
		await view.rerender(
			createElement(StreamingTextEditor, {
				content: `${first}","caption":"unfinished`,
				presentation: "chat",
			}),
		);
		expect(
			document.querySelectorAll("[data-slot=gallery-grid] img"),
		).toHaveLength(2);
		expect(document.querySelector("[data-slot=rich-media-source]")).toBeNull();
		await click(required("[data-slot=rich-media-source-toggle]"));
		expect(required("[data-slot=rich-media-source]").textContent).toContain(
			'"caption":"unfinished',
		);
	});

	test("invalid streaming payload stays a placeholder and settled invalid payload exposes its source", async () => {
		const content = '```recommendations\n{"items":[{"title":invalid';
		const view = await dom.render(
			createElement(StreamingTextEditor, { content, presentation: "chat" }),
		);
		expect(document.querySelector("[data-slot=rich-media-source]")).toBeNull();
		expect(required("[data-slot=rich-media-skeleton]")).toBeTruthy();
		await view.rerender(
			createElement(TextEditor, {
				initialContent: content,
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(required("[data-slot=rich-media-source]").textContent).toContain(
			'"title":invalid',
		);
		expect(
			document.querySelector("[data-slot=rich-media-skeleton]"),
		).toBeNull();
	});

	test("a retained preview resets for edits and settled invalid content", async () => {
		const content = '```gallery\n{"items":[{"src":"/a.svg","alt":"A"}]}';
		const view = await dom.render(
			createElement(StreamingTextEditor, { content, presentation: "chat" }),
		);
		const appended = `${content}\nunexpected`;
		await view.rerender(
			createElement(StreamingTextEditor, {
				content: appended,
				presentation: "chat",
			}),
		);
		expect(required("[data-slot=gallery-grid] img").getAttribute("alt")).toBe(
			"A",
		);
		expect(document.querySelector("pre")).toBeNull();
		await view.rerender(
			createElement(StreamingTextEditor, {
				content: "```gallery\n{",
				presentation: "chat",
			}),
		);
		expect(required("[data-slot=rich-media-skeleton]")).toBeTruthy();
		expect(document.querySelector("[data-slot=gallery-grid]")).toBeNull();
		await view.rerender(
			createElement(TextEditor, {
				initialContent: appended,
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(required("[data-slot=rich-media-source]").textContent).toContain(
			"unexpected",
		);
	});

	test("recommendations retain rank, linked title, image description, badge and specs", async () => {
		await dom.render(
			createElement(TextEditor, {
				initialContent: fence(
					"recommendations",
					JSON.stringify(examples.recommendations),
				),
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(
			document.querySelectorAll("[data-slot=recommendation-list] > li"),
		).toHaveLength(2);
		expect(required("[data-slot=recommendation-rank]").textContent).toBe("1.");
		expect(
			required("[data-slot=recommendation-title] a").getAttribute("href"),
		).toBe("https://example.com/a");
		expect(
			required("[data-slot=recommendation-body] > figure img").getAttribute(
				"alt",
			),
		).toBe("Model A");
		expect(required("[data-slot=recommendation-badge]").textContent).toBe(
			"Best fit",
		);
		expect(required("[data-slot=recommendation-specs] dd").textContent).toBe(
			"400 PS",
		);
	});

	test("comparison bars encode the declared scale and retain readable values", async () => {
		await dom.render(
			createElement(TextEditor, {
				initialContent: fence(
					"comparison",
					JSON.stringify(examples.comparison),
				),
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		const meter = required("[role=meter]");
		expect(meter.getAttribute("aria-valuenow")).toBe("4.5");
		expect(meter.getAttribute("aria-valuemax")).toBe("5");
		expect(meter.getAttribute("aria-valuetext")).toBe("Very good");
		expect(
			required<HTMLElement>("[data-slot=comparison-fill]").style.width,
		).toBe("90%");
		expect(required("[data-slot=comparison-label]").textContent).toBe(
			"AVery good",
		);
	});

	test("failed gallery images leave captions, credits and an accessible placeholder", async () => {
		await dom.render(
			createElement(TextEditor, {
				initialContent: fence("gallery", JSON.stringify(examples.gallery)),
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		await fire(
			required("[data-slot=gallery-grid] img"),
			new dom.window.Event("error") as unknown as Event,
		);
		expect(
			required("[data-slot=rich-media-image-fallback]").getAttribute(
				"aria-label",
			),
		).toBe("Model A");
		expect(required("figcaption").textContent).toContain(
			"A <script> and `code` remain text",
		);
		expect(required("[data-slot=rich-media-credit]").textContent).toBe(
			"Illustration",
		);
	});

	test("incomplete chart JSON keeps source readable without an error panel", async () => {
		const content = '{"data":[';
		await dom.render(
			createElement(
				RendererPresentationContext.Provider,
				{ value: "chat" },
				createElement(ChartCodeBlock, { content, language: "nivo" }),
			),
		);
		expect(required("[data-slot=chart-source]").textContent).toBe(content);
		expect(document.body.textContent).not.toContain("Failed to parse");
		expect(document.querySelector(".bg-destructive\\/10")).toBeNull();
	});
});
