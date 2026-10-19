import * as THREE from "three";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { ShaderPass } from "three/addons/postprocessing/ShaderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import type { Post } from "./internal";

/**
 * The bloom lights only the hottest HDR pixels: the pulse heads on the lit
 * execution wires, so they glow faintly; windows and the board stay crisp.
 */
const BLOOM = { strength: 0.4, threshold: 1.25, radius: 0.2 } as const;
/** Dither in the dark, so its soft gradients never band; 8-bit steps. */
const GRAIN = 1.2 / 255;

const FINAL_VERT = /* glsl */ `
varying vec2 vUv;
void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }
`;

const FINAL_FRAG = /* glsl */ `
uniform sampler2D tDiffuse;
uniform float uGrain;
uniform float uTime;
varying vec2 vUv;
float hash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
vec3 toSRGB(vec3 c) {
  c = max(c, vec3(0.0));
  return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), c));
}
vec3 rolloff(vec3 c) {
  float m = max(max(c.r, c.g), c.b);
  const float k = 0.985;
  if (m > k) {
    float nm = k + (1.0 - k) * (1.0 - exp(-(m - k) / (1.0 - k)));
    vec3 h = c * (nm / m);
    float w = smoothstep(2.0, 9.0, m) * 0.5;
    c = mix(h, vec3(nm), w);
  }
  return c;
}
void main() {
  vec3 o = toSRGB(rolloff(texture2D(tDiffuse, vUv).rgb));
  o += (hash(gl_FragCoord.xy + fract(uTime * 7.31) * 131.0) - 0.5) * uGrain;
  gl_FragColor = vec4(o, 1.0);
}
`;

interface Passes {
	composer: EffectComposer;
	bloom: UnrealBloomPass;
	final: ShaderPass;
	samples: number;
}

// MSAA only where the device's pixels are coarse; at DPR 1.5 and above the
// density hides aliasing. A lowered render scale never adds it: that would make
// the frames it is meant to speed up slower.
const samplesFor = (deviceDpr: number) => (deviceDpr >= 1.5 ? 0 : 4);

function buildPasses(
	renderer: THREE.WebGLRenderer,
	scene: THREE.Scene,
	camera: THREE.Camera,
	size: { w: number; h: number; dpr: number; samples: number },
): Passes {
	const { w, h, dpr, samples } = size;
	const rt = new THREE.WebGLRenderTarget(
		Math.round(w * dpr),
		Math.round(h * dpr),
		{ type: THREE.HalfFloatType, samples },
	);
	const composer = new EffectComposer(renderer, rt);
	composer.setPixelRatio(dpr);
	composer.setSize(w, h);
	composer.addPass(new RenderPass(scene, camera));
	const bloom = new UnrealBloomPass(
		new THREE.Vector2(w * dpr, h * dpr),
		0.5,
		0.5,
		1.0,
	);
	for (const target of bloomTargets(bloom)) target.depthBuffer = false;
	composer.addPass(bloom);
	const final = new ShaderPass({
		uniforms: {
			tDiffuse: { value: null },
			uGrain: { value: 0 },
			uTime: { value: 0 },
		},
		vertexShader: FINAL_VERT,
		fragmentShader: FINAL_FRAG,
	});
	// It draws to the screen and nothing reads its output, so the composer
	// never needs its second full-size target.
	final.needsSwap = false;
	composer.addPass(final);
	return { composer, bloom, final, samples };
}

const bloomTargets = (bloom: UnrealBloomPass) => [
	bloom.renderTargetBright,
	...bloom.renderTargetsHorizontal,
	...bloom.renderTargetsVertical,
];

function disposePasses(p: Passes) {
	p.composer.dispose();
	p.bloom.dispose();
	p.final.dispose();
}

/** Every material the passes draw with, each on a full-screen quad of its own. */
function passMaterials(p: Passes) {
	const { bloom, final } = p;
	return [
		bloom.materialHighPassFilter,
		...bloom.separableBlurMaterials,
		bloom.compositeMaterial,
		bloom.blendMaterial,
		final.material,
	];
}

export function createPost(
	renderer: THREE.WebGLRenderer,
	scene: THREE.Scene,
	camera: THREE.Camera,
): Post {
	let passes: Passes | null = null;
	return {
		get active() {
			return passes !== null;
		},
		// Size and pixel-ratio changes resize the targets in place; only a
		// change of the MSAA sample count needs new ones.
		resize(w, h, dpr, deviceDpr) {
			const samples = samplesFor(deviceDpr);
			if (passes && passes.samples === samples) {
				passes.composer.setPixelRatio(dpr);
				passes.composer.setSize(w, h);
				return;
			}
			if (passes) disposePasses(passes);
			passes = buildPasses(renderer, scene, camera, { w, h, dpr, samples });
		},
		// The bloom materials render into half-float targets and the final pass
		// to the screen; each compiles in the state it draws in, so first use
		// reuses the program instead of linking it on the main thread.
		async compile() {
			if (!passes) return;
			const [final, ...offscreen] = passMaterials(passes).reverse();
			const quad = (material: THREE.Material) =>
				new THREE.Mesh(new THREE.PlaneGeometry(2, 2), material);
			const draw = new THREE.Scene().add(...offscreen.map(quad));
			const shown = new THREE.Scene().add(quad(final));
			const target = renderer.getRenderTarget();
			renderer.setRenderTarget(passes.composer.readBuffer);
			const drawn = renderer.compileAsync(draw, camera);
			renderer.setRenderTarget(null);
			const screen = renderer.compileAsync(shown, camera);
			renderer.setRenderTarget(target);
			await Promise.all([drawn, screen]);
			for (const s of [draw, shown]) {
				for (const mesh of s.children as THREE.Mesh[]) mesh.geometry.dispose();
			}
		},
		// Frees the targets while nothing is drawn; the next frame allocates them again.
		release() {
			if (!passes) return;
			passes.composer.renderTarget1.dispose();
			passes.composer.renderTarget2.dispose();
			for (const target of bloomTargets(passes.bloom)) target.dispose();
		},
		// The bloom only runs on the dark stage, where the selected window's rim
		// and the pulse heads glow; on the porcelain every rim is painted crisp.
		update(f) {
			if (!passes) return;
			const bloom = passes.bloom;
			bloom.enabled = f.reveal.sky > 0.5;
			bloom.strength = BLOOM.strength;
			bloom.threshold = BLOOM.threshold;
			bloom.radius = BLOOM.radius;
			const fu = passes.final.uniforms;
			fu.uGrain.value = f.reveal.sky * GRAIN;
			fu.uTime.value = f.t;
		},
		render() {
			passes?.composer.render();
		},
		warm() {
			if (!passes) return;
			const on = passes.bloom.enabled;
			passes.bloom.enabled = true;
			passes.composer.render();
			passes.bloom.enabled = on;
		},
		dispose() {
			if (passes) disposePasses(passes);
			passes = null;
		},
	};
}
