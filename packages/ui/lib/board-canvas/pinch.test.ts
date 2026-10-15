import { expect, test } from "bun:test";
import { pinchViewport } from "./pinch";

test("pinch scales and translates around the original world centroid", () => {
	const anchor = { x: 300, y: -40 };
	const center = { x: 640, y: 350 };
	const viewport = pinchViewport(0.5, 100, anchor, center, 160, 0.02, 3);
	expect(viewport.zoom).toBe(0.8);
	expect(viewport.x + anchor.x * viewport.zoom).toBe(center.x);
	expect(viewport.y + anchor.y * viewport.zoom).toBe(center.y);
});

test("zoom limits retain centroid anchoring and coincident fingers stay finite", () => {
	for (const distance of [0, 1, 1000]) {
		const viewport = pinchViewport(
			1,
			0,
			{ x: 10, y: 20 },
			{ x: 200, y: 150 },
			distance,
			0.02,
			3,
		);
		expect(viewport.zoom).toBeGreaterThanOrEqual(0.02);
		expect(viewport.zoom).toBeLessThanOrEqual(3);
		expect(viewport.x + 10 * viewport.zoom).toBe(200);
		expect(viewport.y + 20 * viewport.zoom).toBe(150);
	}
});
