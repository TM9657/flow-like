import * as THREE from "three";
import type { Reveal, SceneContext, Surface } from "./internal";
import {
	COLORS,
	GLSL_DOTS,
	GLSL_FOG,
	GLSL_SDF,
	PLANE_VERT,
	PREMULTIPLIED,
	createResources,
	fogOf,
	lerp,
	mixSrgb,
	smooth,
} from "./shared";

/**
 * The drain front (world units from the nearest dock) at the start and the
 * end of the glass beat, and the half width of its soft edge: narrow, so the
 * floor is porcelain or dark glass and never a grey haze between them.
 */
const DRAIN = { from: -2, to: 46, soft: 0.9 } as const;
/** Opacity of the glass once drained, at the start of the beat and at its end. */
const GLASS_ALPHA = { from: 0.32, to: 0.06 } as const;
/** The dark beneath the floor, the board's own background colour. */
const DEPTH_Y = -30;
/** The floor reaches past the camera's far plane, so nothing shows beyond its edge. */
const FLOOR_SIZE = 1400;
/** Fog density on the porcelain and on the dark stage. */
const FOG = { light: 0.021, dark: 0.024 } as const;

// The floor. On the porcelain it is the page itself: exact background, the dot
// grid, contact shadows and a fine ember slot under every dock. A front then
// spreads from the docks, outward and toward the camera: behind it the floor
// is dark glass that lets the operating layer beneath show through. Its edge
// is narrow, and drained floor fogs toward the dark, so the light never turns
// to a grey haze on its way out. On the glass only the windows the story
// touches keep a short, soft glow at the dock their threads leave from.
const FLOOR_FRAG = /* glsl */ `
uniform vec4 uWinA[8];
uniform vec4 uWinB[8];
uniform float uDrain;
uniform float uSoft;
uniform float uFollow;
uniform float uGlassAlpha;
uniform float uTime;
uniform float uMotion;
uniform vec3 uBg;
uniform vec3 uDot;
uniform vec3 uShadow;
uniform vec3 uGlass;
varying vec3 vWorld;
${GLSL_FOG}
${GLSL_DOTS}
${GLSL_SDF}
const float SLOT = 0.86;
const float DOCK = 0.24;
vec2 toLocal(vec2 v, float c, float s) {
  return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

void main() {
  vec2 p = vWorld.xz;
  vec3 ember = vec3(1.0, 0.25, 0.07);
  vec2 dpx = dFdx(p);
  vec2 dpy = dFdy(p);
  float ao = 0.0;
  float contact = 0.0;
  float slot = 0.0;
  float dock = 0.0;
  float glow = 0.0;
  float foot = 0.0;
  float near = 1e3;
  for (int i = 0; i < 8; i++) {
    vec4 a = uWinA[i];
    vec4 b = uWinB[i];
    float c = cos(a.w);
    float s = sin(a.w);
    vec2 l = toLocal(p - a.xy, c, s);
    float hw = a.z;
    float ds = sdRound(l, vec2(hw * SLOT, 0.008), 0.008);
    near = min(near, max(ds, 0.0));
    if (ds > hw + 1.5) continue;
    vec2 lx = toLocal(dpx, c, s);
    vec2 ly = toLocal(dpy, c, s);
    float pwx = max(abs(lx.x) + abs(ly.x), 1e-4);
    float pw = max(abs(lx.y) + abs(ly.y), 1e-4);
    float h = max(b.y, 0.0);
    float lean = b.x;
    float energy = 0.3 + b.z * 0.9 + b.w * 1.6;
    float story = b.z * 0.9 + b.w * 1.6;
    float wEff = max(0.008, pw * 0.8);
    float line = (1.0 - smoothstep(wEff - pw * 0.5, wEff + pw * 0.5, abs(l.y))) * (0.008 / wEff);
    slot += line * (1.0 - smoothstep(hw * SLOT - pwx, hw * SLOT + pwx, abs(l.x))) * energy;
    float dd = sdRound(l, vec2(hw * DOCK, 0.008), 0.008);
    dock += line * (1.0 - smoothstep(hw * DOCK - pwx * 4.0, hw * DOCK + pwx * 4.0, abs(l.x))) * story;
    glow += exp(-max(dd, 0.0) * 8.0) * story;
    foot += exp(-max(ds, 0.0) * 9.0) * energy;
    float dFoot = sdRound(l - vec2(0.0, -lean * 0.5), vec2(hw * 0.97, lean * 0.5 + 0.015), 0.05);
    ao += (1.0 - smoothstep(-0.05, 0.16 + h * 0.9, dFoot)) * 0.075 / (1.0 + h * 4.0);
    float dBase = sdRound(l - vec2(0.0, 0.01), vec2(hw * 0.985, 0.012), 0.012);
    contact += (1.0 - smoothstep(0.0, 0.035 + h * 0.6, dBase)) * 0.2 / (1.0 + h * 6.0);
  }
  // The front leads from the docks; once the sky has turned at the horizon
  // the rest of the floor follows it.
  float drain = max(1.0 - smoothstep(uDrain - uSoft, uDrain + uSoft, near), uFollow);
  float breathe = 0.9 + 0.1 * sin(uTime * 1.4) * uMotion;
  float fog = fogAmount(vWorld);
  vec3 porcelain = mix(uBg, uDot, dots(p, 0.2, 0.0125) * 0.9);
  porcelain = mix(porcelain, uShadow, clamp(ao + contact, 0.0, 0.55));
  vec3 glass = uGlass * uGlassAlpha;
  vec3 surf = mix(porcelain, glass, drain);
  float alpha = mix(1.0, uGlassAlpha, drain);
  vec3 lit = ember * slot * (0.55 + 0.5 * breathe) * 0.7 + ember * foot * 0.02;
  vec3 glassLit = ember * (dock * 0.5 + glow * 0.045) * breathe;
  vec3 emit = mix(lit, glassLit, drain);
  vec3 haze = mix(uFogColor, uGlass, drain);
  vec3 pm = mix(surf, haze, fog) + emit * (1.0 - fog);
  gl_FragColor = vec4(pm, mix(alpha, 1.0, fog));
}
`;

/**
 * The sky darkens from the horizon upward: a front (NDC heights above the
 * horizon) that starts `lead` below it and leaves past the frame's top edge,
 * with a narrow soft edge of half width `soft`. The undrained floor's fog takes
 * the sky's colour at the horizon, so the two always meet without a seam.
 */
const SKY = { lead: 0.04, soft: 0.1 } as const;
/** CSS px from the stage's top of the header's bottom edge: its theme follows the sky there. */
const HEADER_LINE = 64;

const SKY_VERT = /* glsl */ `
void main() {
  gl_Position = vec4(position.xy, 0.0, 1.0);
}
`;

const SKY_FRAG = /* glsl */ `
uniform vec2 uViewport;
uniform float uHorizon;
uniform float uFront;
uniform float uSoft;
uniform vec3 uLight;
uniform vec3 uDark;
vec3 toLinear(vec3 c) {
  return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}
void main() {
  float y = gl_FragCoord.y / uViewport.y * 2.0 - 1.0;
  float dark = 1.0 - smoothstep(uFront - uSoft, uFront + uSoft, y - uHorizon);
  gl_FragColor = vec4(toLinear(mix(uLight, uDark, dark)), 1.0);
}
`;

export function createSurface(ctx: SceneContext): Surface {
	const { scene, shared, camera, renderer } = ctx;
	const res = createResources(ctx.maxAniso);
	scene.background = null;

	const uniforms = {
		uWinA: shared.uWinA,
		uWinB: shared.uWinB,
		uDrain: { value: DRAIN.from as number },
		uSoft: { value: DRAIN.soft },
		uFollow: { value: 0 },
		uGlassAlpha: { value: GLASS_ALPHA.from as number },
		uTime: shared.uTime,
		uMotion: shared.uMotion,
		uBg: { value: COLORS.bg.clone() },
		uDot: { value: COLORS.dot.clone() },
		uShadow: { value: COLORS.shadow.clone() },
		uGlass: { value: COLORS.bgDark.clone() },
		...fogOf(shared),
	};
	const floor = new THREE.Mesh(
		res.track(new THREE.PlaneGeometry(FLOOR_SIZE, FLOOR_SIZE)),
		res.material({
			...PREMULTIPLIED,
			uniforms,
			vertexShader: PLANE_VERT,
			fragmentShader: FLOOR_FRAG,
		}),
	);
	floor.rotation.x = -Math.PI / 2;
	// After everything beneath, before the window reflections (4) that lie on it.
	floor.renderOrder = 3.5;
	const depth = new THREE.Mesh(
		res.track(new THREE.PlaneGeometry(FLOOR_SIZE, FLOOR_SIZE)),
		res.track(new THREE.MeshBasicMaterial({ color: COLORS.bgDark })),
	);
	depth.rotation.x = -Math.PI / 2;
	depth.position.y = DEPTH_Y;
	const light = COLORS.bg.clone().convertLinearToSRGB();
	const darkSrgb = COLORS.bgDark.clone().convertLinearToSRGB();
	const skyUniforms = {
		uViewport: { value: new THREE.Vector2(1, 1) },
		uHorizon: { value: 0 },
		uFront: { value: -(SKY.lead + SKY.soft) },
		uSoft: { value: SKY.soft },
		uLight: { value: new THREE.Vector3(light.r, light.g, light.b) },
		uDark: { value: new THREE.Vector3(darkSrgb.r, darkSrgb.g, darkSrgb.b) },
	};
	// Drawn first behind everything, filling the frame: the sky.
	const backdrop = new THREE.Mesh(
		res.track(new THREE.PlaneGeometry(2, 2)),
		res.material({
			uniforms: skyUniforms,
			vertexShader: SKY_VERT,
			fragmentShader: SKY_FRAG,
			depthTest: false,
			depthWrite: false,
		}),
	);
	backdrop.frustumCulled = false;
	backdrop.renderOrder = -10;
	scene.add(backdrop, floor, depth);

	const ahead = new THREE.Vector3();
	const far = new THREE.Vector3();
	const size = new THREE.Vector2();
	/** NDC y (with the lens shift) of the horizon straight ahead of the camera. */
	function horizon() {
		camera.getWorldDirection(ahead);
		ahead.y = 0;
		if (ahead.lengthSq() < 1e-8) ahead.set(0, 0, -1);
		ahead.normalize();
		far.copy(camera.position).addScaledVector(ahead, 400);
		far.y = camera.position.y;
		return far.project(camera).y;
	}
	const darkAt = (y: number, front: number) =>
		1 - smooth(front - SKY.soft, front + SKY.soft, y);

	return {
		// The floor turns first, from the docks out to the frame's foot and the
		// far floor; then the sky turns from the horizon up past the header.
		update(reveal: Reveal) {
			const level = horizon();
			const start = -(SKY.lead + SKY.soft);
			const end = Math.max(start + 0.1, 1 - level + SKY.soft);
			const front = lerp(start, end, reveal.skyFront);
			const atHorizon = darkAt(0, front);
			renderer.getDrawingBufferSize(size);
			skyUniforms.uViewport.value.copy(size);
			skyUniforms.uHorizon.value = level;
			skyUniforms.uFront.value = front;
			mixSrgb(shared.uFogColor.value, COLORS.bg, COLORS.bgDark, atHorizon);
			shared.uFogDensity.value = lerp(FOG.light, FOG.dark, reveal.sky);
			uniforms.uDrain.value = lerp(DRAIN.from, DRAIN.to, reveal.glass);
			uniforms.uFollow.value = atHorizon;
			uniforms.uGlassAlpha.value = lerp(
				GLASS_ALPHA.from,
				GLASS_ALPHA.to,
				reveal.glass,
			);
			// Until the floor drains, nothing beneath can show.
			depth.visible = reveal.glass > 0;
			const line = 1 - (2 * HEADER_LINE) / Math.max(1, ctx.stage.height);
			return darkAt(line - level, front);
		},
		dispose: () => res.dispose(),
	};
}
