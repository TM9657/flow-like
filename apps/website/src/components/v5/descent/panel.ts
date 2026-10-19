import * as THREE from "three";
import type { FogUniforms, PanelUniforms } from "./internal";
import { GLSL_FOG, GLSL_SDF } from "./shared";

export const PANEL_VERT = /* glsl */ `
varying vec3 vObj;
varying vec3 vN;
varying vec3 vWorld;
varying vec3 vWN;
void main() {
  vObj = position;
  vN = normal;
  vec4 wp = modelMatrix * vec4(position, 1.0);
  vWorld = wp.xyz;
  vWN = normalize(mat3(modelMatrix) * normal);
  gl_Position = projectionMatrix * viewMatrix * wp;
}
`;

// App windows (uKind 0 light, 1 dark). Face colours stay below the bloom
// threshold; only the selection rim and arrival flashes on the dark stage are
// emissive, so copy never washes out.
export const PANEL_FRAG = /* glsl */ `
uniform sampler2D uMap;
uniform vec2 uSize;
uniform float uRadius;
uniform float uKind;
uniform float uDark;
uniform float uSel;
uniform float uFlash;
uniform float uHover;
uniform float uFocus;
uniform float uMirror;
uniform float uMirrorFade;
uniform vec4 uHL;
uniform float uHLAmt;
varying vec3 vObj;
const float RIM_SOFT_PX = 4.0;
varying vec3 vN;
varying vec3 vWorld;
varying vec3 vWN;
${GLSL_FOG}
${GLSL_SDF}

// The region an arrival changed: tinted, with a glowing edge.
vec3 highlight(vec3 col, vec2 uv, float inside, inout float emit) {
  float hx = smoothstep(uHL.x - 0.01, uHL.x + 0.01, uv.x) * (1.0 - smoothstep(uHL.z - 0.01, uHL.z + 0.01, uv.x));
  float hy = smoothstep(uHL.y - 0.01, uHL.y + 0.01, 1.0 - uv.y) * (1.0 - smoothstep(uHL.w - 0.01, uHL.w + 0.01, 1.0 - uv.y));
  float hl = hx * hy * inside;
  float edge = min(min(uv.x - uHL.x, uHL.z - uv.x), min((1.0 - uv.y) - uHL.y, uHL.w - (1.0 - uv.y)));
  emit += hx * hy * (1.0 - smoothstep(0.0, 0.012, edge)) * uHLAmt * 1.6;
  return mix(col, col * vec3(1.0, 0.8, 0.72) + vec3(0.05, 0.012, 0.0), hl * uHLAmt * 0.6);
}

vec3 face(vec3 V, vec2 uv, float sd, inout float emit) {
  vec4 ui = texture(uMap, uv);
  float inside = (1.0 - smoothstep(-0.004, 0.003, sd)) * ui.a;
  float topLight = smoothstep(-0.6, 0.6, vObj.y / uSize.y);
  vec3 glass = mix(vec3(0.905, 0.89, 0.88), vec3(0.985, 0.98, 0.975), topLight);
  float outerEdge = smoothstep(0.03, 0.0, abs(sd - 0.052));
  glass = mix(glass, vec3(0.99), outerEdge * 0.8);
  float innerShade = smoothstep(0.016, 0.0, sd) * step(0.0, sd);
  glass *= 1.0 - innerShade * 0.12;
  vec3 col = mix(glass, ui.rgb, inside);
  if (uHLAmt > 0.001) col = highlight(col, uv, inside, emit);
  float sheenAxis = dot(vObj.xy / uSize, vec2(0.62, 0.78)) + dot(V, vec3(0.35, 0.0, 0.0));
  float sheen = smoothstep(0.22, 0.0, abs(sheenAxis - 0.18)) * (uKind > 0.5 ? 0.012 : 0.04);
  col += sheen * (1.0 - uDark);
  col = min(col, vec3(0.985));
  // On the dark stage a window reads as a lit screen, a touch warmer. It never
  // passes the bloom threshold: work arriving lights the rim, not the face.
  vec3 screen = col * vec3(1.0, 0.975, 0.95) * (0.975 + 0.025 * uSel);
  return mix(col, screen, uDark);
}

void main() {
  if (uMirror > 0.5 ? vWorld.y > 0.0 : vWorld.y < 0.0) discard;
  vec3 N = normalize(vWN);
  vec3 V = normalize(cameraPosition - vWorld);
  vec3 on = normalize(vN);
  vec3 ember = vec3(1.0, 0.25, 0.07);
  vec2 uv = vObj.xy / uSize + 0.5;
  float sd = sdRound(vObj.xy, uSize * 0.5, uRadius);
  float emit = 0.0;
  vec3 col;
  if (on.z > 0.55) {
    col = face(V, uv, sd, emit);
  } else if (on.z < -0.55) {
    col = mix(vec3(0.9, 0.885, 0.875), vec3(0.2, 0.17, 0.16), uDark);
  } else {
    float up = clamp(N.y * 0.5 + 0.5, 0.0, 1.0);
    col = mix(vec3(0.86, 0.85, 0.845), vec3(0.985, 0.98, 0.975), up);
    col = mix(col, vec3(0.46, 0.34, 0.29), uDark * 0.8);
  }
  float outer = sdRound(vObj.xy, uSize * 0.5 + 0.055, uRadius + 0.055);
  float px = max(fwidth(outer), 1e-4);
  float rimLine = 1.0 - smoothstep(px * 0.6, px * 1.8 + 0.004, abs(outer + 0.007));
  // The selection is a crisp ember rim on both stages, so a window stays sharp.
  float ind = clamp(uSel + uFlash * 0.45 + uHover * 0.4, 0.0, 1.0);
  col = mix(col, vec3(0.97, 0.36, 0.2), rimLine * ind);
  // Keyboard focus on the window's control: a ring three pixels wide inside the
  // edge, in the page's focus colour (--primary, in linear light).
  float focusRing = 1.0 - smoothstep(px * 1.5, px * 2.5, abs(outer + px * 2.0 + 0.004));
  col = mix(col, vec3(0.965, 0.093, 0.026), focusRing * uFocus);
  // On the dark stage work arriving also lights a soft band several pixels
  // wide around the rim, which a one-pixel line could not hold.
  float band = exp(-abs(outer + 0.007) / (RIM_SOFT_PX * px));
  emit += band * uFlash * 0.5 * uDark;
  col += ember * emit;
  if (uMirror > 0.5) {
    float f = exp(-(-vWorld.y) * 3.2) * uMirrorFade;
    gl_FragColor = vec4(col * f, f);
    return;
  }
  // Only the porcelain world hazes the windows; on the dark stage they stay crisp.
  col = mix(col, uFogColor, fogAmount(vWorld) * 0.6 * (1.0 - uDark));
  gl_FragColor = vec4(col, 1.0);
}
`;

export function panelUniforms(
	map: THREE.CanvasTexture,
	size: THREE.Vector2,
	radius: number,
	kind: number,
	fog: FogUniforms,
): PanelUniforms {
	return {
		uMap: { value: map },
		uSize: { value: size },
		uRadius: { value: radius },
		uKind: { value: kind },
		uDark: { value: 0 },
		uSel: { value: 0 },
		uFlash: { value: 0 },
		uHover: { value: 0 },
		uFocus: { value: 0 },
		uMirror: { value: 0 },
		uMirrorFade: { value: 1 },
		uHL: { value: new THREE.Vector4(0, 0, 1, 1) },
		uHLAmt: { value: 0 },
		...fog,
	};
}
