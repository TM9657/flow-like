import * as THREE from "three";
import type {
	FogUniforms,
	Resources,
	Reveal,
	SharedUniforms,
} from "./internal";

export const ARC_RADIUS = 12;

export const COLORS = {
	bg: new THREE.Color("#fcf9f8"),
	bgDark: new THREE.Color("#0c0e13"),
	dot: new THREE.Color("#e6dfda"),
	shadow: new THREE.Color("#4a3c34"),
};

/**
 * Scroll progress p of the pinned stage at which each beat of the story happens.
 * Each surface turns as a narrow front, so every frame is porcelain or dark
 * glass with one soft edge between them: first the floor, then the sky.
 */
export const TIMELINE = {
	/** The windows rest on the porcelain until here. */
	rest: 0.12,
	/** The camera pulls back to the whole arc and the board beneath it, mostly before the glass. */
	pull: [0.12, 0.34],
	/** The floor drains to dark glass, from the docks outward. */
	glass: [0.17, 0.29],
	/** Then the sky darkens, from the horizon up past the frame's top. */
	sky: [0.24, 0.31],
	/** The window reflections fade from the floor before the glass reaches them. */
	reflections: [0.12, 0.2],
	/** The board fades up beneath the glass as it drains. */
	board: [0.18, 0.32],
	/** The threads draw in from the docks, the centre windows first. */
	threads: [0.32, 0.5],
	/** The active route lights up on the board. */
	lit: [0.4, 0.5],
	/** The layer comes alive: the scenario (re)starts and the windows show their states. */
	alive: 0.4,
	/** The copy band fades in while the layer comes alive (controller.ts fades it over the same range). */
	band: [0.46, 0.62],
} as const;

export const clamp = (v: number, a = 0, b = 1) => Math.min(b, Math.max(a, v));
export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const smooth = (a: number, b: number, v: number) => {
	const t = clamp((v - a) / (b - a));
	return t * t * (3 - 2 * t);
};
export const easeOut = (t: number) => 1 - (1 - t) ** 3;
const within = (range: readonly [number, number], p: number) =>
	smooth(range[0], range[1], p);
// Jumps up to a new flash level, then fades at `rate` per second.
export const decay = (
	current: number,
	target: number,
	dt: number,
	rate: number,
) => (dt <= 0 ? target : Math.max(target, current - dt * rate));
// 1 → 0 over `length` seconds after an event, 0 before it.
export const pulse = (since: number, length: number) =>
	since >= 0 && since < length ? 1 - since / length : 0;

/** Reveal amounts at progress p, written into `out` (no allocation per frame). */
export function revealInto(p: number, out: Reveal): Reveal {
	out.glass = within(TIMELINE.glass, p);
	out.sky = within(TIMELINE.sky, p);
	out.skyFront =
		clamp((p - TIMELINE.sky[0]) / (TIMELINE.sky[1] - TIMELINE.sky[0])) ** 2;
	out.reflections = 1 - within(TIMELINE.reflections, p);
	out.board = within(TIMELINE.board, p);
	out.threads = clamp(
		(p - TIMELINE.threads[0]) / (TIMELINE.threads[1] - TIMELINE.threads[0]),
	);
	out.lit = within(TIMELINE.lit, p);
	out.band = within(TIMELINE.band, p);
	return out;
}

export const createReveal = (): Reveal => ({
	glass: 0,
	sky: 0,
	skyFront: 0,
	reflections: 1,
	board: 0,
	threads: 0,
	lit: 0,
	band: 0,
});

export const GLSL_FOG = /* glsl */ `
uniform vec3 uFogColor;
uniform float uFogDensity;
float fogAmount(vec3 wp) {
  float d = length(cameraPosition - wp);
  float f = uFogDensity * d;
  return 1.0 - exp(-f * f);
}
`;

export const GLSL_SDF = /* glsl */ `
float sdRound(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
`;

export const GLSL_DOTS = /* glsl */ `
float dots(vec2 p, float spacing, float radius) {
  vec2 g = (fract(p / spacing) - 0.5) * spacing;
  float d = length(g);
  float aa = fwidth(d) * 1.2 + 1e-4;
  float fade = 1.0 - smoothstep(0.1, 0.32, fwidth(p.x) / spacing);
  return (1.0 - smoothstep(radius - aa, radius + aa, d)) * fade;
}
`;

export const PLANE_VERT = /* glsl */ `
varying vec3 vWorld;
void main() {
  vec4 wp = modelMatrix * vec4(position, 1.0);
  vWorld = wp.xyz;
  gl_Position = projectionMatrix * viewMatrix * wp;
}
`;

// Premultiplied output, so the transparent surface never shows colour halos.
export const PREMULTIPLIED = {
	transparent: true,
	depthWrite: false,
	blending: THREE.CustomBlending,
	blendSrc: THREE.OneFactor,
	blendDst: THREE.OneMinusSrcAlphaFactor,
} as const;

export function createSharedUniforms(reducedMotion: boolean): SharedUniforms {
	return {
		uTime: { value: 0 },
		uFogColor: { value: COLORS.bg.clone() },
		uFogDensity: { value: 0.021 },
		uMotion: { value: reducedMotion ? 0 : 1 },
		uWinA: { value: Array.from({ length: 8 }, () => new THREE.Vector4()) },
		uWinB: { value: Array.from({ length: 8 }, () => new THREE.Vector4()) },
	};
}

/** The world's fog, shared by reference. */
export const fogOf = (shared: SharedUniforms): FogUniforms => ({
	uFogColor: shared.uFogColor,
	uFogDensity: shared.uFogDensity,
});

export function createResources(maxAniso: number): Resources {
	const disposables: { dispose(): void }[] = [];
	const track = <T extends { dispose(): void }>(o: T): T => {
		disposables.push(o);
		return o;
	};
	const material = (params: THREE.ShaderMaterialParameters) =>
		track(new THREE.ShaderMaterial(params));
	return {
		track,
		material,
		canvasTexture: (c) => {
			const t = track(new THREE.CanvasTexture(c));
			t.colorSpace = THREE.SRGBColorSpace;
			t.anisotropy = maxAniso;
			t.minFilter = THREE.LinearMipmapLinearFilter;
			t.generateMipmaps = true;
			return t;
		},
		dispose: () => {
			for (const d of disposables) d.dispose();
			disposables.length = 0;
		},
	};
}

// Texture storage is immutable, so a redraw at a new size (a tag measured
// with the final font) must reallocate it.
export function swapImage(tex: THREE.CanvasTexture, c: HTMLCanvasElement) {
	if (tex.image.width !== c.width || tex.image.height !== c.height) {
		tex.dispose();
	}
	tex.image = c;
	tex.needsUpdate = true;
}

const ca = new THREE.Color();
const cb = new THREE.Color();
/** Linear-light colour `out` between `a` and `b`, mixed in sRGB as the page mixes colours. */
export function mixSrgb(
	out: THREE.Color,
	a: THREE.Color,
	b: THREE.Color,
	t: number,
): THREE.Color {
	ca.copy(a).convertLinearToSRGB();
	cb.copy(b).convertLinearToSRGB();
	return out
		.setRGB(lerp(ca.r, cb.r, t), lerp(ca.g, cb.g, t), lerp(ca.b, cb.b, t))
		.convertSRGBToLinear();
}
