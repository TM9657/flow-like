import { describe, expect, test } from "bun:test";
import { createFrameBudget } from "./frame-budget";

const feed = (
	b: ReturnType<typeof createFrameBudget>,
	ms: number,
	n: number,
) => {
	const changes: number[] = [];
	for (let i = 0; i < n; i++) {
		const next = b.sample(ms);
		if (next !== null) changes.push(next);
	}
	return changes;
};

describe("frame budget", () => {
	test("on-time 60 Hz frames never step down", () => {
		const b = createFrameBudget();
		expect(feed(b, 16.7, 2000)).toEqual([]);
		expect(b.scale).toBe(1);
	});
	test("on-time 120 Hz frames never step down", () => {
		const b = createFrameBudget();
		expect(feed(b, 8.3, 2000)).toEqual([]);
	});
	test("sustained 33 ms frames at 60 Hz step down to the floor and stop", () => {
		const b = createFrameBudget(0.6);
		const mixed = createFrameBudget(0.6);
		// Mostly dropped frames, some on vsync so the vsync estimate is right.
		let changes: number[] = [];
		for (let i = 0; i < 3000; i++) {
			const next = mixed.sample(i % 10 === 0 ? 16.7 : 33.4);
			if (next !== null) changes.push(next);
		}
		expect(changes.length).toBeGreaterThan(0);
		expect(Math.min(...changes)).toBeCloseTo(0.6, 5);
		changes = feed(b, 40, 3000);
		expect(b.scale).toBeCloseTo(0.6, 5);
	});
	test("120 Hz frames over 14 ms step down", () => {
		const b = createFrameBudget();
		let changes: number[] = [];
		for (let i = 0; i < 400; i++) {
			const next = b.sample(i % 10 === 0 ? 8.3 : 15.5);
			if (next !== null) changes.push(next);
		}
		expect(changes[0]).toBeCloseTo(0.85, 5);
		changes = [];
		expect(b.scale).toBeLessThan(1);
	});
	test("steps back up slowly once frames are on time", () => {
		const b = createFrameBudget();
		for (let i = 0; i < 200; i++) b.sample(i % 10 === 0 ? 8.3 : 15.5);
		const low = b.scale;
		expect(low).toBeLessThan(1);
		const ups = feed(b, 8.3, 90 * 5);
		expect(ups).toEqual([]);
		const later = feed(b, 8.3, 90 * 6);
		expect(later.length).toBe(1);
		expect(b.scale).toBeCloseTo(low + 0.1, 5);
	});
	test("pauses and outliers are ignored", () => {
		const b = createFrameBudget();
		expect(feed(b, 1200, 500)).toEqual([]);
		expect(feed(b, 0, 500)).toEqual([]);
		expect(b.scale).toBe(1);
	});
});
