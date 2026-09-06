import { describe, expect, it } from "bun:test";
import { MAX_HOME_LAYOUT_BYTES } from "./home-layout";
import {
	formatHomeLayoutJson,
	homeLayoutFingerprint,
	homeLayoutsEqual,
	parseHomeLayoutJson,
	serializeHomeLayout,
} from "./home-layout-json";
import type { IHomeLayout } from "./types";

const layout: IHomeLayout = {
	version: 1,
	title: "Operations",
	widgets: [
		{
			id: "status",
			type: "information",
			title: "Status",
			size: { columns: 6, rows: 3 },
			appearance: { variant: "card", accent: "neutral" },
			config: { body: "Everything is running." },
		},
	],
};

describe("home layout JSON transfer", () => {
	it("serializes and parses a portable layout", () => {
		const source = serializeHomeLayout(layout);
		expect(source).toContain('\n  "widgets": [');
		expect(parseHomeLayoutJson(source)).toEqual({ ok: true, layout });
	});

	it("normalizes optional widget fields before applying pasted JSON", () => {
		const result = parseHomeLayoutJson(
			JSON.stringify({
				version: 1,
				widgets: [{ id: "future", type: "future-widget" }],
			}),
		);
		expect(result).toEqual({
			ok: true,
			layout: {
				version: 1,
				widgets: [
					{
						id: "future",
						type: "future-widget",
						size: { columns: 6, rows: 3 },
						appearance: { variant: "card", accent: "neutral" },
						config: {},
					},
				],
			},
		});
	});

	it("compares layouts by their persisted JSON value", () => {
		const left = structuredClone(layout);
		const right = structuredClone(layout);
		left.widgets[0].config = { first: 1, second: 2 };
		right.widgets[0].config = { second: 2, first: 1 };
		expect(homeLayoutsEqual(left, right)).toBe(true);
		right.widgets[0].title = "Different";
		expect(homeLayoutsEqual(left, right)).toBe(false);
	});

	it("fingerprints structurally equal config objects consistently", () => {
		const left = structuredClone(layout);
		const right = structuredClone(layout);
		left.widgets[0].config = { first: 1, second: 2 };
		right.widgets[0].config = { second: 2, first: 1 };
		expect(homeLayoutFingerprint(left)).toBe(homeLayoutFingerprint(right));
		right.widgets[0].config.second = 3;
		expect(homeLayoutFingerprint(left)).not.toBe(homeLayoutFingerprint(right));
	});

	it("rejects malformed JSON and layouts with ambiguous widget identity", () => {
		expect(parseHomeLayoutJson("{")).toMatchObject({
			ok: false,
			error: expect.stringContaining("Invalid JSON"),
		});
		expect(
			parseHomeLayoutJson(
				JSON.stringify({
					...layout,
					widgets: [layout.widgets[0], layout.widgets[0]],
				}),
			),
		).toMatchObject({
			ok: false,
			error: expect.stringContaining("unique id"),
		});
	});

	it("rejects a pasted layout that cannot fit the save limit", () => {
		const oversized = structuredClone(layout);
		oversized.widgets[0].config.body = "x".repeat(MAX_HOME_LAYOUT_BYTES);
		expect(parseHomeLayoutJson(JSON.stringify(oversized))).toEqual({
			ok: false,
			error: "This layout exceeds the 128 KiB save limit.",
		});
	});

	it("rejects text that the backend cannot persist", () => {
		const invalidValues: Array<{
			update: (value: IHomeLayout) => void;
			error: string;
		}> = [
			{
				update: (value) => {
					value.title = "x".repeat(257);
				},
				error: "Layout title must not exceed 256 bytes.",
			},
			{
				update: (value) => {
					value.widgets[0].id = "é".repeat(65);
				},
				error: "Widget 1 id must not exceed 128 bytes.",
			},
			{
				update: (value) => {
					value.widgets[0].type = "x".repeat(81);
				},
				error: "Widget 1 type must not exceed 80 bytes.",
			},
			{
				update: (value) => {
					value.widgets[0].appearance.variant = "";
				},
				error: "Widget 1 appearance variant must contain 1 to 80 bytes.",
			},
			{
				update: (value) => {
					value.widgets[0].description = "x".repeat(2001);
				},
				error: "Widget 1 description must not exceed 2000 bytes.",
			},
		];

		for (const invalid of invalidValues) {
			const value = structuredClone(layout);
			invalid.update(value);
			expect(parseHomeLayoutJson(JSON.stringify(value))).toEqual({
				ok: false,
				error: invalid.error,
			});
		}
	});

	it("rejects numeric overflow before formatting or applying", () => {
		const source = JSON.stringify({
			...layout,
			widgets: [{ ...layout.widgets[0], config: { score: 1 } }],
		}).replace('"score":1', '"score":1e400');
		expect(parseHomeLayoutJson(source)).toMatchObject({
			ok: false,
			error: expect.stringContaining("finite"),
		});
		expect(formatHomeLayoutJson(source)).toMatchObject({
			ok: false,
			error: expect.stringContaining("finite"),
		});
	});
});
