import assert from "node:assert/strict";
import test from "node:test";
import { percentile, summarizeFrames } from "./measurement.mjs";

test("frame summaries count missed refresh opportunities independently of measured speed", () => {
	const result = summarizeFrames([10, 10, 20, 40], 10);
	assert.equal(result.p50Ms, 15);
	assert.ok(Math.abs(result.p95Ms - 37) < 1e-9);
	assert.equal(result.missedFrames, 4);
	assert.equal(result.missedFrameRatio, 0.5);
	assert.equal(result.frameCount, 4);
});

test("empty measurements cannot imply perfect frame times", () => {
	assert.equal(percentile([], 0.95), null);
	assert.equal(summarizeFrames([], 1000 / 60).p95Ms, null);
});
