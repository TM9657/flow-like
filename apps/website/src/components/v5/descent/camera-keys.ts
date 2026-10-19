import * as THREE from "three";
import type { CameraKey, LayoutKind, Vec3Tuple } from "./internal";
import { TIMELINE } from "./shared";

/**
 * The camera never leaves the light side of the floor. It rests inside the
 * window arc, then pulls back and rises a little while the floor turns to
 * glass, until the whole arc and the board beneath it are in view; from there
 * it holds while the layer comes alive and the copy settles.
 */
export const cameraKeys: Record<LayoutKind, readonly CameraKey[]> = {
	portrait: [
		{ p: 0, pos: [0, 1.9, 10.4], at: [0, 1.85, 0], fov: 46 },
		{ p: 0.06, pos: [0, 1.95, 10.45], at: [0, 1.78, 0.05], fov: 46 },
		{ p: TIMELINE.rest, pos: [0, 2, 10.55], at: [0, 1.72, 0.08], fov: 46 },
		{ p: TIMELINE.glass[1], pos: [0, 2.9, 16.5], at: [0, 0.35, 0], fov: 46 },
		{ p: 1, pos: [0, 2.9, 16.5], at: [0, 0.35, 0], fov: 46 },
	],
	landscape: [
		{ p: 0, pos: [0, 2, 12.4], at: [0, 1.16, 0.4], fov: 36 },
		{ p: 0.06, pos: [0, 2.07, 12.5], at: [0, 1.09, 0.45], fov: 36 },
		{ p: TIMELINE.rest, pos: [0, 2.13, 12.68], at: [0, 1.03, 0.5], fov: 36 },
		{ p: TIMELINE.glass[1], pos: [0, 3, 28], at: [0, 0.1, 0], fov: 36 },
		{ p: 1, pos: [0, 3, 28], at: [0, 0.1, 0], fov: 36 },
	],
};

/** Aspect ratio each key list was authored for, and the intro's lift. */
export const LENS: Record<LayoutKind, { intro: Vec3Tuple; refAspect: number }> =
	{
		portrait: { intro: [0, 1.0, 4.5], refAspect: 0.462 },
		landscape: { intro: [0, 1.4, 5.5], refAspect: 1.6 },
	};

type Field = "pos" | "at" | "fov";
type Tangents = Record<Field, Float64Array>;

const keyValue = (k: CameraKey, field: Field, j: number) =>
	field === "fov" ? k.fov : k[field][j];

// Fritsch–Carlson tangents for every key and component, computed once per key list.
const tangentCache = new WeakMap<readonly CameraKey[], Tangents>();
function tangentsOf(keys: readonly CameraKey[]): Tangents {
	const cached = tangentCache.get(keys);
	if (cached) return cached;
	const n = keys.length;
	const build = (field: Field, dims: number) => {
		const out = new Float64Array(n * dims);
		const slope = (a: number, b: number, j: number) =>
			(keyValue(keys[b], field, j) - keyValue(keys[a], field, j)) /
			(keys[b].p - keys[a].p);
		for (let k = 0; k < n; k++) {
			for (let j = 0; j < dims; j++) {
				let m: number;
				if (k === 0) m = slope(0, 1, j);
				else if (k === n - 1) m = slope(n - 2, n - 1, j);
				else {
					const s0 = slope(k - 1, k, j);
					const s1 = slope(k, k + 1, j);
					m = s0 * s1 <= 0 ? 0 : (2 * s0 * s1) / (s0 + s1);
				}
				out[k * dims + j] = m;
			}
		}
		return out;
	};
	const t = { pos: build("pos", 3), at: build("at", 3), fov: build("fov", 1) };
	tangentCache.set(keys, t);
	return t;
}

/** Index of the key segment that holds p, or -1 / -2 before the first or after the last key. */
function segmentOf(keys: readonly CameraKey[], p: number) {
	const n = keys.length;
	if (p <= keys[0].p) return -1;
	if (p >= keys[n - 1].p) return -2;
	let i = 0;
	while (i < n - 2 && p > keys[i + 1].p) i++;
	return i;
}

/** Monotone cubic (Fritsch–Carlson) through the keys, so no key overshoots; writes into `out`. */
export function sampleKeysInto<T extends number[]>(
	keys: readonly CameraKey[],
	p: number,
	field: Field,
	out: T,
): T {
	const dims = field === "fov" ? 1 : 3;
	out.length = dims;
	const i = segmentOf(keys, p);
	if (i < 0) {
		const key = i === -1 ? keys[0] : keys[keys.length - 1];
		for (let j = 0; j < dims; j++) out[j] = keyValue(key, field, j);
		return out;
	}
	const h = keys[i + 1].p - keys[i].p;
	const t = (p - keys[i].p) / h;
	const t2 = t * t;
	const t3 = t2 * t;
	const m = tangentsOf(keys)[field];
	for (let j = 0; j < dims; j++) {
		const y0 = keyValue(keys[i], field, j);
		const y1 = keyValue(keys[i + 1], field, j);
		const m0 = m[i * dims + j] * h;
		const m1 = m[(i + 1) * dims + j] * h;
		out[j] =
			(2 * t3 - 3 * t2 + 1) * y0 +
			(t3 - 2 * t2 + t) * m0 +
			(-2 * t3 + 3 * t2) * y1 +
			(t3 - t2) * m1;
	}
	return out;
}

export function sampleKeys(
	keys: readonly CameraKey[],
	p: number,
	field: Field,
) {
	return sampleKeysInto(keys, p, field, []);
}

// Narrower screens widen the field of view so the composition keeps its width.
export function fitFov(baseFov: number, refAspect: number, aspect: number) {
	if (aspect >= refAspect) return baseFov;
	const half = Math.tan(THREE.MathUtils.degToRad(baseFov) / 2);
	return THREE.MathUtils.radToDeg(2 * Math.atan(half * (refAspect / aspect)));
}

/** Field of view after a framing zoom: tan(fov / 2) is divided by `zoom`. */
export function zoomFov(fov: number, zoom: number) {
	const half = Math.tan(THREE.MathUtils.degToRad(fov) / 2);
	return THREE.MathUtils.radToDeg(2 * Math.atan(half / zoom));
}

const fovOut: number[] = [0];
/** The key field of view at p for a stage aspect, before any framing. */
export function keyFov(kind: LayoutKind, p: number, aspect: number) {
	sampleKeysInto(cameraKeys[kind], p, "fov", fovOut);
	return fitFov(fovOut[0], LENS[kind].refAspect, aspect);
}
