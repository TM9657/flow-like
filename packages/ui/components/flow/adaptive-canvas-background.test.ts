import { describe, expect, test } from "bun:test";
import {
	canvasGridLayout,
	canvasGridMultiplier,
} from "./adaptive-canvas-background";

describe("native background grid density", () => {
	test("keeps the original gap and phase at detailed zoom", () => {
		for (const zoom of [2 / 3, 1, 2, 3]) {
			expect(canvasGridMultiplier(zoom)).toBe(1);
			expect(canvasGridLayout(canvasGridMultiplier(zoom))).toEqual({
				gap: 12,
				dotOffset: undefined,
			});
		}
	});

	test("bounds screen density using the smallest power-of-two multiplier", () => {
		for (let i = 0; i <= 1000; i++) {
			const zoom = 0.02 + (i / 1000) * (2 / 3 - 0.02);
			const multiplier = canvasGridMultiplier(zoom);
			const screenGap = canvasGridLayout(multiplier).gap * zoom;
			expect(Number.isInteger(Math.log2(multiplier))).toBe(true);
			expect(screenGap).toBeGreaterThanOrEqual(8 - 1e-10);
			expect(screenGap).toBeLessThan(16 + 1e-10);
		}
		expect(canvasGridMultiplier(0.02)).toBe(64);
	});

	test("changes only at discrete zoom thresholds", () => {
		for (const multiplier of [1, 2, 4, 8, 16, 32]) {
			const threshold = 8 / (12 * multiplier);
			expect(canvasGridMultiplier(threshold)).toBe(multiplier);
			expect(canvasGridMultiplier(threshold * 0.999)).toBe(multiplier * 2);
			expect(canvasGridMultiplier(threshold * 1.001)).toBe(multiplier);
		}
	});

	test("coarse tiers keep one world origin and retain a subset of finer dots", () => {
		for (const multiplier of [2, 4, 8, 16, 32]) {
			const fine = canvasGridLayout(multiplier);
			const coarse = canvasGridLayout(multiplier * 2);
			expect(coarse.dotOffset).toBe(fine.dotOffset);
			expect(coarse.gap).toBe(fine.gap * 2);
			if (coarse.dotOffset === undefined || fine.dotOffset === undefined)
				throw new Error("Coarse grids need an explicit world offset");
			for (const index of [-2, -1, 0, 1, 2]) {
				const coarseCenter = index * coarse.gap + 0.5 - coarse.dotOffset;
				const fineCenter = index * 2 * fine.gap + 0.5 - fine.dotOffset;
				expect(coarseCenter).toBe(fineCenter);
			}
		}
	});
});
