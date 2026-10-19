import * as THREE from "three";
import type { Resources, Uniform } from "./internal";
import { PREMULTIPLIED } from "./shared";

/** A point as [x, y, z] world units: the shape of board-pulse's paths. */
export type WorldPoint = readonly number[];

/** sRGB channels in 0–1. Strokes blend in gamma space, as egui and Canvas2D do. */
export type Srgb = readonly [number, number, number];

export interface RibbonView {
	/** CSS px per board unit on the board plane; stroke widths are board units. */
	readonly zoom: Uniform<number>;
	/** Drawing size in CSS px. */
	readonly viewport: Uniform<THREE.Vector2>;
	readonly dpr: Uniform<number>;
	/** Fades every stroke in with the reveal. */
	readonly visibility: Uniform<number>;
}

export interface RibbonBatch {
	readonly mesh: THREE.Mesh;
	clear(): void;
	/** A stroke through world points; `alpha` is one value or one per point. */
	stroke(
		points: readonly WorldPoint[],
		color: Srgb,
		alpha: number | readonly number[],
		width: number,
	): void;
	/**
	 * A disc of `radius`: "hard" has an antialiased edge, "soft" fades out like a
	 * glow. `heat` scales the colour past white, so the bloom catches it.
	 */
	disc(
		at: WorldPoint,
		radius: number,
		color: Srgb,
		alpha: number,
		edge?: "hard" | "soft",
		heat?: number,
	): void;
	/** Uploads what was drawn since clear(). */
	commit(): void;
}

// Strokes are expanded in screen space, so widths stay in pixels at any depth.
// Joints are mitred within one strip, so a translucent stroke never overlaps
// itself and cannot bead the way per-segment lines do.
const RIBBON_VERT = /* glsl */ `
attribute vec3 aPrev;
attribute vec3 aNext;
attribute vec2 aOffset;
attribute vec4 aColor;
attribute float aWidth;
attribute float aMode;
uniform vec2 uViewport;
uniform float uZoom;
uniform float uDpr;
varying vec4 vColor;
varying vec2 vCoord;
varying float vHalf;
varying float vMode;

vec2 screenOf(vec4 clip) {
  return clip.xy / clip.w * 0.5 * uViewport;
}

void main() {
  mat4 mvp = projectionMatrix * modelViewMatrix;
  vec4 clip = mvp * vec4(position, 1.0);
  float px = 1.0 / uDpr;
  vec2 offset;
  if (aMode > 0.5) {
    float radius = aWidth * uZoom;
    float reach = radius + px;
    offset = aOffset * reach;
    vCoord = offset;
    vHalf = radius;
  } else {
    vec2 here = screenOf(clip);
    vec2 back = here - screenOf(mvp * vec4(aPrev, 1.0));
    vec2 ahead = screenOf(mvp * vec4(aNext, 1.0)) - here;
    float lb = length(back);
    float la = length(ahead);
    vec2 t1 = la > 1e-4 ? ahead / la : back / max(lb, 1e-4);
    vec2 t0 = lb > 1e-4 ? back / lb : t1;
    vec2 n0 = vec2(-t0.y, t0.x);
    vec2 n1 = vec2(-t1.y, t1.x);
    vec2 miter = n0 + n1;
    float ml = length(miter);
    miter = ml > 1e-4 ? miter / ml : n1;
    float stretch = 1.0 / max(dot(miter, n1), 0.4);
    float half_ = 0.5 * aWidth * uZoom;
    float reach = max(half_, 0.5 * px) + px;
    offset = miter * reach * stretch * aOffset.x;
    vCoord = vec2(aOffset.x * reach, 0.0);
    vHalf = half_;
  }
  clip.xy += offset / (0.5 * uViewport) * clip.w;
  vColor = aColor;
  vMode = aMode;
  gl_Position = clip;
}
`;

const RIBBON_FRAG = /* glsl */ `
uniform float uDpr;
uniform float uVis;
varying vec4 vColor;
varying vec2 vCoord;
varying float vHalf;
varying float vMode;

vec3 toLinear(vec3 c) {
  return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}

void main() {
  float px = 1.0 / uDpr;
  float d = vMode > 0.5 ? length(vCoord) : abs(vCoord.x);
  float cover;
  if (vMode > 1.5) {
    float r = max(vHalf, 1e-3);
    cover = exp(-3.0 * d * d / (r * r)) * (1.0 - smoothstep(0.85 * r, r, d));
  } else {
    float half_ = max(vHalf, 0.5 * px);
    cover = clamp((half_ + 0.5 * px - d) / px, 0.0, 1.0) * min(1.0, vHalf / (0.5 * px));
  }
  float a = vColor.a * cover * uVis;
  if (a < 0.002) discard;
  gl_FragColor = vec4(toLinear(vColor.rgb * a), a);
}
`;

const CORNERS: readonly (readonly [number, number])[] = [
	[-1, -1],
	[1, -1],
	[-1, 1],
	[1, 1],
];

export function createRibbonView(): RibbonView {
	return {
		zoom: { value: 1 },
		viewport: { value: new THREE.Vector2(1, 1) },
		dpr: { value: 1 },
		visibility: { value: 1 },
	};
}

function createBuffers(res: Resources, capacity: number) {
	const geometry = res.track(new THREE.BufferGeometry());
	const attribute = (name: string, size: number) => {
		const attr = new THREE.BufferAttribute(
			new Float32Array(capacity * size),
			size,
		).setUsage(THREE.DynamicDrawUsage);
		geometry.setAttribute(name, attr);
		return attr;
	};
	const buffers = {
		position: attribute("position", 3),
		prev: attribute("aPrev", 3),
		next: attribute("aNext", 3),
		offset: attribute("aOffset", 2),
		color: attribute("aColor", 4),
		width: attribute("aWidth", 1),
		mode: attribute("aMode", 1),
	};
	const index = new THREE.BufferAttribute(
		new Uint16Array(capacity * 3),
		1,
	).setUsage(THREE.DynamicDrawUsage);
	geometry.setIndex(index);
	geometry.setDrawRange(0, 0);
	return { geometry, index, ...buffers, all: Object.values(buffers) };
}

// Strokes lie on the board plane, so they are pulled toward the camera to win
// the depth test against it while windows in front still hide them.
const createMaterial = (res: Resources, view: RibbonView) =>
	res.material({
		...PREMULTIPLIED,
		side: THREE.DoubleSide,
		polygonOffset: true,
		polygonOffsetFactor: -2,
		polygonOffsetUnits: -8,
		uniforms: {
			uViewport: view.viewport,
			uZoom: view.zoom,
			uDpr: view.dpr,
			uVis: view.visibility,
		},
		vertexShader: RIBBON_VERT,
		fragmentShader: RIBBON_FRAG,
	});

const apart = (a: WorldPoint, b: WorldPoint) =>
	Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2]) > 1e-6;

/**
 * How many points a strip keeps: repeated points are dropped, so every
 * segment has a direction.
 */
function distinctCount(points: readonly WorldPoint[]) {
	let count = 0;
	let last = -1;
	for (let i = 0; i < points.length; i++) {
		if (last >= 0 && !apart(points[last], points[i])) continue;
		count++;
		last = i;
	}
	return count;
}

const alphaAt = (alpha: number | readonly number[], i: number) =>
	typeof alpha === "number" ? alpha : (alpha[i] ?? 0);

/** Index of the next point apart from point `i`, or `i` itself at the end. */
function nextApart(points: readonly WorldPoint[], i: number) {
	for (let j = i + 1; j < points.length; j++) {
		if (apart(points[i], points[j])) return j;
	}
	return i;
}

/** One draw call holding many strokes and discs, rebuilt by the caller when they change. */
export function createRibbons(
	res: Resources,
	view: RibbonView,
	capacity: number,
): RibbonBatch {
	const b = createBuffers(res, capacity);
	const mesh = new THREE.Mesh(b.geometry, createMaterial(res, view));
	mesh.frustumCulled = false;
	/** What the next vertices are drawn with. */
	const pen = { r: 0, g: 0, b: 0, alpha: 0, width: 0, mode: 0 };
	let vertices = 0;
	let indices = 0;

	const fits = (more: number, moreIndices: number) =>
		vertices + more <= capacity && indices + moreIndices <= capacity * 3;

	function hold(rgb: Srgb, width: number, mode: number, heat = 1) {
		pen.r = rgb[0] * heat;
		pen.g = rgb[1] * heat;
		pen.b = rgb[2] * heat;
		pen.width = width;
		pen.mode = mode;
	}

	function vertex(
		at: WorldPoint,
		before: WorldPoint,
		after: WorldPoint,
		ox: number,
		oy: number,
	) {
		const v = vertices++;
		b.position.setXYZ(v, at[0], at[1], at[2]);
		b.prev.setXYZ(v, before[0], before[1], before[2]);
		b.next.setXYZ(v, after[0], after[1], after[2]);
		b.offset.setXY(v, ox, oy);
		b.color.setXYZW(v, pen.r, pen.g, pen.b, pen.alpha);
		b.width.setX(v, pen.width);
		b.mode.setX(v, pen.mode);
	}

	// Both edge vertices of strip point `i`, mitred between the kept point
	// before it (`last`, or none) and the next point apart from it.
	function segmentEnds(points: readonly WorldPoint[], last: number, i: number) {
		const before = points[last >= 0 ? last : i];
		const after = points[nextApart(points, i)];
		vertex(points[i], before, after, -1, 0);
		vertex(points[i], before, after, 1, 0);
	}

	// Two triangles over vertices a, a+1 (one end) and a+2, a+3 (the other).
	function quad(a: number) {
		const index = b.index;
		index.setX(indices++, a);
		index.setX(indices++, a + 1);
		index.setX(indices++, a + 2);
		index.setX(indices++, a + 1);
		index.setX(indices++, a + 3);
		index.setX(indices++, a + 2);
	}

	return {
		mesh,
		clear() {
			vertices = 0;
			indices = 0;
		},
		stroke(points, rgb, alpha, width) {
			const n = distinctCount(points);
			if (n < 2 || !fits(n * 2, (n - 1) * 6)) return;
			hold(rgb, width, 0);
			const first = vertices;
			let last = -1;
			for (let i = 0; i < points.length; i++) {
				if (last >= 0 && !apart(points[last], points[i])) continue;
				pen.alpha = alphaAt(alpha, i);
				segmentEnds(points, last, i);
				last = i;
			}
			for (let i = 0; i + 1 < n; i++) quad(first + i * 2);
		},
		disc(at, radius, rgb, alpha, edge = "hard", heat = 1) {
			if (!fits(4, 6)) return;
			hold(rgb, radius, edge === "soft" ? 2 : 1, heat);
			pen.alpha = alpha;
			const first = vertices;
			for (const [ox, oy] of CORNERS) vertex(at, at, at, ox, oy);
			quad(first);
		},
		commit() {
			for (const attr of b.all) {
				attr.clearUpdateRanges();
				attr.addUpdateRange(0, Math.max(1, vertices) * attr.itemSize);
				attr.needsUpdate = true;
			}
			b.index.clearUpdateRanges();
			b.index.addUpdateRange(0, Math.max(1, indices));
			b.index.needsUpdate = true;
			b.geometry.setDrawRange(0, indices);
		},
	};
}
