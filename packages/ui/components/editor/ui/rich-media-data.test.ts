import { describe, expect, test } from "bun:test";
import {
	parseRichMediaData,
	parseStreamingRichMediaData,
} from "./rich-media-data";

describe("rich media JSON", () => {
	test("streaming retains completed cards while the following item is incomplete", () => {
		const data = parseStreamingRichMediaData(
			'{"items":[{"src":"/a.svg","alt":"A"},{"src":"/b.svg","alt":"unfinished',
			"gallery",
		);
		expect(data).toMatchObject({
			kind: "gallery",
			items: [{ src: "/a.svg", alt: "A" }],
		});
		expect(data?.items).toHaveLength(1);
		const recommendation = parseStreamingRichMediaData(
			'{"items":[{"title":"Model A","description":"unfinished',
			"recommendations",
		);
		expect(recommendation).toMatchObject({
			items: [{ title: "Model A", description: undefined }],
		});
	});

	test("streaming comparisons never guess unfinished values or scales", () => {
		expect(
			parseStreamingRichMediaData(
				'{"max":5,"items":[{"label":"A","value":4',
				"comparison",
			),
		).toBeNull();
		expect(
			parseStreamingRichMediaData(
				'{"max":5,"items":[{"label":"A","value":4,',
				"comparison",
			),
		).toMatchObject({ max: 5, items: [{ label: "A", value: 4 }] });
		expect(
			parseStreamingRichMediaData(
				'{"items":[{"label":"A","value":4}],"max":5',
				"comparison",
			),
		).toBeNull();
		expect(
			parseStreamingRichMediaData(
				'{"max":5,"items":[{"label":"A","value":8,',
				"comparison",
			),
		).toBeNull();
	});

	test("a recommendation stays visible at every prefix while optional image and specs arrive", () => {
		const source = JSON.stringify({
			items: [
				{
					title: "Model A",
					image: { src: "/a.svg", alt: "A", caption: "Image caption" },
					specs: [
						{ label: "Power", value: "400 PS" },
						{ label: "Drive", value: "AWD" },
					],
					description: "Description arrives last",
				},
			],
		});
		const start = source.indexOf('"Model A"') + '"Model A"'.length;
		for (let length = start; length <= source.length; length++) {
			const data = parseStreamingRichMediaData(
				source.slice(0, length),
				"recommendations",
			);
			expect(data?.items[0]).toMatchObject({
				id: "recommendation-0",
				title: "Model A",
				rank: 1,
			});
		}
	});

	test("streaming item identities and ranks retain their source position", () => {
		const source =
			'{"items":[{}, {"title":"Second"}, {"title":"Third","image":{';
		const data = parseStreamingRichMediaData(source, "recommendations");
		expect(data).toMatchObject({
			items: [
				{ id: "recommendation-1", title: "Second", rank: 2 },
				{ id: "recommendation-2", title: "Third", rank: 3 },
			],
		});
		const gallery = parseStreamingRichMediaData(
			'{"items":[{}, {"src":"/b.svg","alt":"B"}, {"src":',
			"gallery",
		);
		expect(gallery?.items).toMatchObject([
			{ id: "image-1", src: "/b.svg", alt: "B" },
		]);
	});
	test("preserves gallery captions, credits and relative image sources without a browser", () => {
		const data = parseRichMediaData(
			JSON.stringify({
				title: "Gallery",
				items: [
					{
						src: "/assets/a.svg",
						alt: "A",
						caption: "<b>Plain text</b>",
						credit: "Illustration",
						href: "https://example.com/a",
					},
				],
			}),
			"gallery",
		);
		expect(data).toMatchObject({
			kind: "gallery",
			title: "Gallery",
			items: [
				{
					src: "/assets/a.svg",
					alt: "A",
					caption: "<b>Plain text</b>",
					credit: "Illustration",
					href: "https://example.com/a",
				},
			],
		});
	});

	test("rejects executable or unsupported URLs while keeping descriptions readable", () => {
		for (const src of [
			"javascript:alert(1)",
			"java\tscript:alert(1)",
			"data:text/html,<h1>bad</h1>",
			"data:image/svg+xml,<svg onload='alert(1)'/>",
			"file:///etc/passwd",
		]) {
			const data = parseRichMediaData(
				JSON.stringify({
					items: [{ src, alt: "Image description", href: src }],
				}),
				"gallery",
			);
			expect(data.items[0]).toMatchObject({
				src: undefined,
				href: undefined,
				alt: "Image description",
			});
		}
	});

	test("recommendations preserve supplied order, text and specifications", () => {
		const data = parseRichMediaData(
			JSON.stringify({
				items: [
					{
						title: "A",
						badge: "Best fit",
						description: "Contains `code` and <script> as text",
						image: { src: "storage://app/photos/a.png", alt: "A" },
						specs: [{ label: "Power", value: "400 PS" }],
					},
					{ title: "B" },
				],
			}),
			"recommendations",
		);
		expect(data).toMatchObject({
			kind: "recommendations",
			items: [
				{
					title: "A",
					badge: "Best fit",
					specs: [{ label: "Power", value: "400 PS" }],
				},
				{ title: "B" },
			],
		});
	});

	test("comparison values require a finite declared scale without coercion or clamping", () => {
		expect(
			parseRichMediaData(
				'{"max":5,"items":[{"label":"A","value":4.5,"valueLabel":"Very good"}]}',
				"comparison",
			),
		).toMatchObject({
			max: 5,
			items: [{ value: 4.5, valueLabel: "Very good" }],
		});
		for (const max of [0, -1, "5", null])
			expect(() =>
				parseRichMediaData(
					JSON.stringify({ max, items: [{ label: "A", value: 1 }] }),
					"comparison",
				),
			).toThrow();
		for (const value of [-1, 6, "4", null])
			expect(() =>
				parseRichMediaData(
					JSON.stringify({ max: 5, items: [{ label: "A", value }] }),
					"comparison",
				),
			).toThrow();
		expect(() =>
			parseRichMediaData(
				'{"max":1e999,"items":[{"label":"A","value":1}]}',
				"comparison",
			),
		).toThrow();
	});

	test("invalid and incomplete content never becomes a partial misleading preview", () => {
		for (const content of [
			"{",
			"[]",
			"null",
			'{"items":[]}',
			'{"items":[{"src":"/a.png"}]}',
		])
			expect(() => parseRichMediaData(content, "gallery")).toThrow();
		expect(() =>
			parseRichMediaData(
				'{"items":[{"title":"A","specs":[{"label":"Power","value":400}]}]}',
				"recommendations",
			),
		).toThrow();
	});
});
