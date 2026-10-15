import { describe, expect, test } from "bun:test";
import { miniMapProjection, sameMiniMapRectangles } from "./egui-mini-map";

describe("cached minimap projection", () => {
	test("presentation-only object changes retain pixels while visible graph changes invalidate them", () => {
		const before = [
			{ x: 0, y: 0, width: 220, height: 80, color: "var(--primary)" },
			{ x: 400, y: 120, width: 220, height: 110, color: "orange" },
		];
		expect(sameMiniMapRectangles(before, structuredClone(before))).toBe(true);
		for (const change of [
			{ x: 1 },
			{ y: 1 },
			{ width: 221 },
			{ height: 81 },
			{ color: "var(--tertiary)" },
		])
			expect(
				sameMiniMapRectangles(before, [{ ...before[0], ...change }, before[1]]),
			).toBe(false);
		expect(sameMiniMapRectangles(before, before.slice(1))).toBe(false);
		expect(sameMiniMapRectangles(before, [...before].reverse())).toBe(false);
	});
	test("contains both graph bounds and a viewport outside the graph", () => {
		const result = miniMapProjection(
			{ x: 0, y: 0, width: 10000, height: 5000 },
			{ x: -2000, y: 6000, width: 1000, height: 800 },
		);
		expect(result.x).toBeLessThan(-2000);
		expect(result.y).toBeLessThan(0);
		expect(result.x + result.width).toBeGreaterThan(10000);
		expect(result.y + result.height).toBeGreaterThan(6800);
	});
	test("moving within graph bounds preserves the cached node image transform", () => {
		const bounds = { x: 0, y: 0, width: 10000, height: 8000 };
		expect(
			miniMapProjection(bounds, { x: 10, y: 20, width: 1000, height: 800 }),
		).toEqual(
			miniMapProjection(bounds, { x: 500, y: 600, width: 1000, height: 800 }),
		);
	});
	test("an empty graph still has a finite viewport projection", () => {
		const result = miniMapProjection(undefined, {
			x: 20,
			y: 30,
			width: 0,
			height: 0,
		});
		expect(Object.values(result).every(Number.isFinite)).toBe(true);
		expect(result.width).toBeGreaterThan(0);
	});
});
