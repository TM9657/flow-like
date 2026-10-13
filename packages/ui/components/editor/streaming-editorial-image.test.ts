import { describe, expect, test } from "bun:test";
import { type TElement, createSlateEditor } from "platejs";
import { RICH_REMARK_PLUGINS, safeDeserialize } from "../ui/text-editor";
import { BaseEditorKit } from "./editor-base-kit";
import {
	normalizeStreamingEditorialImage,
	safeStreamingImageUrl,
} from "./streaming-editorial-image";

const worker = createSlateEditor({ plugins: BaseEditorKit, nodeId: false });
const parse = (markdown: string) =>
	safeDeserialize(worker, markdown, true, RICH_REMARK_PLUGINS);

describe("streaming editorial images", () => {
	test("keeps completed quoted fields and drops unfinished attribute values", () => {
		const original = 'Before.\n\n<img src="/sedan.svg" alt="Ant';
		const display = normalizeStreamingEditorialImage(original);
		expect(display.pending).toBe(true);
		expect(display.markdown).toBe(
			'Before.\n\n<img src="/sedan.svg" streamingPending="true" />',
		);
		const image = parse(display.markdown).at(-1) as TElement;
		expect(image.type).toBe("img");
		expect(image.url).toBe("/sedan.svg");
		expect(image.streamingPending).toBe("loading");
		expect(image.alt).toBe("");
	});

	test("reveals an image when src and alt finish, before later metadata", () => {
		const display = normalizeStreamingEditorialImage(
			'<img src="/sedan.svg" alt="A sedan" caption="Half',
		);
		const image = parse(display.markdown)[0] as TElement;
		expect(image.streamingPending).toBe("ready");
		expect(image.alt).toBe("A sedan");
		expect(image.caption).toEqual([{ text: "A sedan" }]);
	});

	test("every prefix stays inside Plate and contains only completed attributes", () => {
		const original =
			'<img src="/sedan.svg?a=1&amp;b=2" alt=\'A > B &quot;sedan&quot;\' caption="Finished caption" credit="Photo" />';
		for (let cut = 4; cut < original.length; cut++) {
			const prefix = original.slice(0, cut);
			const display = normalizeStreamingEditorialImage(prefix);
			expect(display.pending).toBe(true);
			const image = parse(display.markdown)[0] as TElement;
			expect(image.type).toBe("img");
			if (image.url) expect(image.url).toBe("/sedan.svg?a=1&b=2");
			if (image.alt) expect(image.alt).toBe('A > B "sedan"');
		}
		expect(normalizeStreamingEditorialImage(original)).toEqual({
			markdown: original,
			pending: false,
		});
		expect(parse(original)[0]).not.toHaveProperty("streamingPending");
	});

	test("allows only inert editorial attributes while pending", () => {
		const display = normalizeStreamingEditorialImage(
			'<img src="/a.svg" alt="A" onerror="alert(1)" style="color:red" caption="Complete" credit="Half',
		);
		expect(display.markdown).not.toContain("onerror");
		expect(display.markdown).not.toContain("style");
		const image = parse(display.markdown)[0] as TElement;
		expect(image.caption).toEqual([{ text: "Complete" }]);
		expect(image).not.toHaveProperty("credit");
	});

	for (const source of [
		'```html\n<img src="/a',
		'~~~~html\n<img src="/a',
		'- ```html\n  <img src="/a',
		'> ```html\n> <img src="/a',
		'`<img src="/a',
		'``example ` <img src="/a',
		'    <img src="/a',
		'\\<img src="/a',
		'<!-- <img src="/a',
		"<div title=\"<img src='/a",
		'<imgSomething src="/a',
		"<img src={getImage()}",
		"<img src=/a",
		'<img src="/a" alt="A" />\nProse',
	]) {
		test(`preserves literal or unsupported source: ${source}`, () => {
			expect(normalizeStreamingEditorialImage(source)).toEqual({
				markdown: source,
				pending: false,
			});
		});
	}

	test("recognizes an image after closed fences, comments and code spans", () => {
		for (const prefix of [
			'```html\n<img src="ignored">\n```\n\n',
			"~~~\nignored\n~~~\n",
			"`<img>`\n\n",
			"<!-- <img> -->\n\n",
		]) {
			const result = normalizeStreamingEditorialImage(
				`${prefix}<img src="/a" alt="A"`,
			);
			expect(result.pending).toBe(true);
			expect(result.markdown.startsWith(prefix)).toBe(true);
		}
	});

	test("unsafe and entity-obfuscated sources remain placeholders", () => {
		for (const source of [
			"javascript:alert(1)",
			"java&#x09;script:alert(1)",
			"data:text/html;base64,abc",
			"data:image/svg+xml;base64,abc",
			"file:///tmp/image.png",
		]) {
			const display = normalizeStreamingEditorialImage(
				`<img src="${source}" alt="A"`,
			);
			const image = parse(display.markdown)[0] as TElement;
			expect(image.type).toBe("img");
			expect(image.url).toBeUndefined();
			expect(image.streamingPending).toBe("loading");
		}
		expect(safeStreamingImageUrl("storage://app/image.png")).toBe(
			"storage://app/image.png",
		);
		expect(safeStreamingImageUrl("data:image/png;base64,YQ==")).toBe(
			"data:image/png;base64,YQ==",
		);
	});
});
