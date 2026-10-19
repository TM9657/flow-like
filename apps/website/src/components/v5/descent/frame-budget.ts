// Adaptive resolution: steps the render scale down when frames
// run long for a sustained window, and back up slowly once they are on time.

/** A window averaging more than this (or 1.35 vsync intervals) is slow... */
export const BUDGET_MS = 14;
/** ...and one averaging more than this always is (below ~45 fps). */
const CEILING_MS = 22;
const WINDOW = 90;
const WARMUP = 45;
const STEP_DOWN = 0.15;
const STEP_UP = 0.1;
const GOOD_WINDOWS = 6;
const round = (v: number) => Math.round(v * 100) / 100;

export interface FrameBudget {
	/** Current scale for the device pixel ratio, 1 at full resolution. */
	readonly scale: number;
	/** Records one frame interval in ms; returns the new scale when it changes, else null. */
	sample(ms: number): number | null;
	/** Starts measuring afresh (after a pause, a resize or a scale change). */
	reset(): void;
}

export function createFrameBudget(minScale = 0.6): FrameBudget {
	const samples = new Float32Array(WINDOW);
	const sorted = new Float32Array(WINDOW);
	let scale = 1;
	let count = 0;
	let skip = WARMUP;
	let good = 0;

	function judge(): number | null {
		sorted.set(samples);
		sorted.sort();
		let sum = 0;
		for (const v of samples) sum += v;
		const mean = sum / WINDOW;
		// The fastest frames approximate the display's vsync interval.
		const vsync = Math.max(6.5, sorted[2]);
		if (mean > Math.min(CEILING_MS, Math.max(BUDGET_MS, vsync * 1.35))) {
			good = 0;
			if (scale <= minScale) return null;
			scale = Math.max(minScale, round(scale - STEP_DOWN));
			return scale;
		}
		good = mean <= vsync * 1.12 ? good + 1 : 0;
		if (good >= GOOD_WINDOWS && scale < 1) {
			good = 0;
			scale = Math.min(1, round(scale + STEP_UP));
			return scale;
		}
		return null;
	}

	return {
		get scale() {
			return scale;
		},
		sample(ms) {
			if (!(ms > 0) || ms > 250) return null;
			if (skip > 0) {
				skip--;
				return null;
			}
			samples[count++] = ms;
			if (count < WINDOW) return null;
			count = 0;
			const next = judge();
			if (next !== null) skip = WARMUP;
			return next;
		},
		reset() {
			count = 0;
			skip = WARMUP;
		},
	};
}
