import { describe, expect, test } from "bun:test";
import { cameraKeys, fitFov, sampleKeys, zoomFov } from "./camera-keys";
import { BAND_IN } from "./controller";
import { TIMELINE } from "./shared";

const STEPS = 400;
const sample = (kind: "landscape" | "portrait", field: "pos" | "at") =>
	Array.from({ length: STEPS + 1 }, (_, i) =>
		sampleKeys(cameraKeys[kind], i / STEPS, field),
	);

describe.each(["landscape", "portrait"] as const)("%s timeline", (kind) => {
	const keys = cameraKeys[kind];

	test("keys are ordered and pass through every key", () => {
		for (let i = 1; i < keys.length; i++)
			expect(keys[i].p).toBeGreaterThan(keys[i - 1].p);
		for (const k of keys) {
			const pos = sampleKeys(keys, k.p, "pos");
			for (let j = 0; j < 3; j++) expect(pos[j]).toBeCloseTo(k.pos[j], 9);
		}
	});

	test("no overshoot between keys", () => {
		for (let i = 0; i < keys.length - 1; i++) {
			const a = keys[i];
			const b = keys[i + 1];
			for (let s = 1; s < 20; s++) {
				const pos = sampleKeys(keys, a.p + ((b.p - a.p) * s) / 20, "pos");
				for (let j = 0; j < 3; j++) {
					expect(pos[j]).toBeGreaterThanOrEqual(
						Math.min(a.pos[j], b.pos[j]) - 1e-9,
					);
					expect(pos[j]).toBeLessThanOrEqual(
						Math.max(a.pos[j], b.pos[j]) + 1e-9,
					);
				}
			}
		}
	});

	test("the camera stays above the floor and in front of the windows: no dive", () => {
		for (const pos of sample(kind, "pos")) {
			expect(pos[1]).toBeGreaterThan(1.5);
			expect(pos[2]).toBeGreaterThan(9);
		}
	});

	test("the camera only pulls back, and only while it pulls back", () => {
		const positions = sample(kind, "pos");
		for (let i = 1; i < positions.length; i++) {
			expect(positions[i][2]).toBeGreaterThanOrEqual(
				positions[i - 1][2] - 1e-9,
			);
		}
		const end = sampleKeys(keys, TIMELINE.pull[1], "pos");
		const last = sampleKeys(keys, 1, "pos");
		for (let j = 0; j < 3; j++) expect(last[j]).toBeCloseTo(end[j], 9);
	});

	test("it always looks down toward the floor, gently", () => {
		const positions = sample(kind, "pos");
		const targets = sample(kind, "at");
		for (let i = 0; i < positions.length; i++) {
			const [px, py, pz] = positions[i];
			const [ax, ay, az] = targets[i];
			const pitch = Math.atan2(py - ay, Math.hypot(ax - px, az - pz));
			expect(pitch).toBeGreaterThanOrEqual(0);
			expect(pitch).toBeLessThan((14 * Math.PI) / 180);
		}
	});
});

describe("lens", () => {
	test("narrower screens widen the field of view to keep the width", () => {
		expect(fitFov(36, 1.6, 2)).toBe(36);
		const half = (deg: number) => Math.tan((deg * Math.PI) / 360);
		expect(half(fitFov(36, 1.6, 1.2)) * 1.2).toBeCloseTo(half(36) * 1.6, 9);
	});
	test("a framing zoom divides tan(fov / 2)", () => {
		const half = (deg: number) => Math.tan((deg * Math.PI) / 360);
		expect(half(zoomFov(36, 2))).toBeCloseTo(half(36) / 2, 9);
	});
});

describe("copy band", () => {
	test("the page fades the band in over the range the scene settles its picture", () => {
		expect([...BAND_IN]).toEqual([...TIMELINE.band]);
	});
});
