import { describe, expect, test } from "bun:test";
import { CanvasPaintBudget } from "./startup";

describe("native first-paint deadline", () => {
	test("hidden startup can exceed the deadline and still get the full foreground budget", () => {
		const budget = new CanvasPaintBudget(0, false);
		expect(budget.expired(12_500, false)).toBe(false);
		expect(budget.expired(12_500, true)).toBe(false);
		expect(budget.expired(22_499, true)).toBe(false);
		expect(budget.expired(22_500, true)).toBe(true);
	});

	test("leaving and returning to a tab pauses, rather than resets, the visible deadline", () => {
		const budget = new CanvasPaintBudget(0, true);
		expect(budget.expired(4_000, true)).toBe(false);
		expect(budget.expired(4_000, false)).toBe(false);
		expect(budget.expired(120_000, true)).toBe(false);
		expect(budget.expired(125_999, true)).toBe(false);
		expect(budget.expired(126_000, true)).toBe(true);
	});

	test("collapsed or CSS-hidden boards do not spend their first-paint budget", () => {
		const budget = new CanvasPaintBudget(0, false);
		expect(budget.expired(60_000, false)).toBe(false);
		expect(budget.expired(60_250, true)).toBe(false);
		expect(budget.expired(60_500, true)).toBe(false);
		expect(budget.expired(60_500, false)).toBe(false);
		expect(budget.expired(180_000, false)).toBe(false);
		expect(budget.expired(180_000, true)).toBe(false);
		expect(budget.expired(189_750, true)).toBe(true);
	});

	test("a continuously drawable renderer still fails at ten seconds", () => {
		const budget = new CanvasPaintBudget(0, true);
		expect(budget.expired(9_999, true)).toBe(false);
		expect(budget.expired(10_000, true)).toBe(true);
	});
});
