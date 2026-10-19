import * as THREE from "three";
import { RoundedBoxGeometry } from "three/addons/geometries/RoundedBoxGeometry.js";
import type {
	Frame,
	PanelUniforms,
	SceneContext,
	WindowCues,
	WindowPose,
	WindowState,
	Windows,
} from "./internal";
import { PANEL_FRAG, PANEL_VERT, panelUniforms } from "./panel";
import {
	ARC_RADIUS,
	PREMULTIPLIED,
	clamp,
	createResources,
	decay,
	easeOut,
	fogOf,
	lerp,
	pulse,
	smooth,
	swapImage,
} from "./shared";
import {
	APP_SPECS,
	ARC_ORDER,
	UNIT,
	arrivalRegion,
	drawAppWindow,
} from "./ui-draw";

const WINDOW_RIM = 0.055;
export const WINDOW_LEAN = THREE.MathUtils.degToRad(13);
export const WINDOW_HOVER = 0.06;
/** Seconds the edge flash and the changed region stay lit after a state change. */
const FLASH_S = 1.25;
const REGION_S = 1.9;
/** How strongly the reduced-motion still holds the glow on a changed region. */
const STILL_GLOW = 0.6;

/** Height of app `app`'s window top above the floor at rest. */
export function windowTop(app: number) {
	const h = APP_SPECS[app].h / UNIT + WINDOW_RIM * 2;
	return WINDOW_HOVER + h * Math.cos(WINDOW_LEAN);
}

/**
 * Corner `i` of app `app`'s window `width` wide standing at `pose` (0, 1 its
 * base, 2, 3 its leaning top, left to right), into `out`.
 */
export function windowCorner(
	app: number,
	width: number,
	pose: Readonly<WindowPose>,
	i: number,
	out: THREE.Vector3,
) {
	const top = i >= 2;
	const lean = top
		? (windowTop(app) - WINDOW_HOVER) * Math.tan(WINDOW_LEAN)
		: 0;
	const half = (width / 2 + WINDOW_RIM) * (i % 2 ? 1 : -1);
	return out.set(
		pose.x - Math.sin(pose.rotY) * lean + Math.cos(pose.rotY) * half,
		top ? windowTop(app) : WINDOW_HOVER,
		pose.z - Math.cos(pose.rotY) * lean - Math.sin(pose.rotY) * half,
	);
}

/** Point on the floor under the dock of a window at `pose`, `offset` along the dock, into `out`. */
export function windowDock(
	pose: Readonly<WindowPose>,
	offset: number,
	out: THREE.Vector3,
) {
	return out.set(
		pose.x + Math.cos(pose.rotY) * offset,
		-0.015,
		pose.z - Math.sin(pose.rotY) * offset,
	);
}

export interface WindowLayout {
	app: number;
	slot: number;
	w: number;
	h: number;
	theta: number;
	x: number;
	z: number;
	rotY: number;
	lean: number;
	hover: number;
}

interface WindowRec {
	i: number;
	w: number;
	h: number;
	rim: number;
	mesh: THREE.Mesh;
	pivot: THREE.Group;
	group: THREE.Group;
	uniforms: PanelUniforms;
	reflUniforms: PanelUniforms;
	refl: THREE.Mesh;
	face: HTMLCanvasElement;
	region: readonly [number, number, number, number] | null;
	/** The state the scenario last set. */
	state: WindowState;
	/** The state the face shows. */
	shown: WindowState;
	/** Scene time the shown state last changed. */
	changed: number;
	sel: number;
	flash: number;
	hover: number;
	focus: number;
}

/** The windows laid out on the arc, indexed by app. */
export function arcWindows(portrait: boolean) {
	const gap = portrait ? 0.26 : 0.36;
	const total =
		ARC_ORDER.reduce((s, a) => s + APP_SPECS[a].w / UNIT, 0) +
		gap * (ARC_ORDER.length - 1);
	let s = -total / 2;
	const windows: WindowLayout[] = [];
	for (const [slot, app] of ARC_ORDER.entries()) {
		const w = APP_SPECS[app].w / UNIT;
		const theta = (s + w / 2) / ARC_RADIUS;
		s += w + gap;
		windows[app] = {
			app,
			slot,
			w,
			h: APP_SPECS[app].h / UNIT,
			theta,
			x: ARC_RADIUS * Math.sin(theta),
			z: ARC_RADIUS - ARC_RADIUS * Math.cos(theta),
			rotY: -theta,
			lean: WINDOW_LEAN,
			hover: WINDOW_HOVER,
		};
	}
	return windows;
}

export function createWindows(ctx: SceneContext): Windows {
	const { scene, world, camera, canvas, data, options, stage, shared } = ctx;
	const res = createResources(ctx.maxAniso);
	const sharedFog = fogOf(shared);

	const faceOf = (state: WindowState) => ({ state, arrivals: data.arrivals });

	function createReflection(
		geo: THREE.BufferGeometry,
		uniforms: PanelUniforms,
	) {
		const reflUniforms: PanelUniforms = {
			...uniforms,
			uMirror: { value: 1 },
			uMirrorFade: { value: 1 },
		};
		const refl = new THREE.Mesh(
			geo,
			res.material({
				...PREMULTIPLIED,
				uniforms: reflUniforms,
				vertexShader: PANEL_VERT,
				fragmentShader: PANEL_FRAG,
			}),
		);
		refl.matrixAutoUpdate = false;
		refl.matrixWorldAutoUpdate = false;
		refl.renderOrder = 4;
		refl.frustumCulled = false;
		scene.add(refl);
		return { refl, reflUniforms };
	}

	function windowMesh(i: number, map: THREE.CanvasTexture, radius: number) {
		const spec = APP_SPECS[i];
		const w = spec.w / UNIT;
		const h = spec.h / UNIT;
		const outerH = h + WINDOW_RIM * 2;
		const geo = res.track(
			new RoundedBoxGeometry(
				w + WINDOW_RIM * 2,
				outerH,
				0.05,
				4,
				Math.min(radius + WINDOW_RIM, outerH / 2 - 0.01),
			),
		);
		const size = new THREE.Vector2(w, h);
		const uniforms = panelUniforms(
			map,
			size,
			radius,
			spec.dark ? 1 : 0,
			sharedFog,
		);
		const mesh = new THREE.Mesh(
			geo,
			res.material({
				uniforms,
				vertexShader: PANEL_VERT,
				fragmentShader: PANEL_FRAG,
			}),
		);
		mesh.userData.app = i;
		return { mesh, geo, uniforms, w, h };
	}

	function createWindow(i: number): WindowRec {
		const spec = APP_SPECS[i];
		const drawn = drawAppWindow(data.apps[i], spec, undefined, faceOf("idle"));
		const map = res.canvasTexture(drawn.canvas);
		const { mesh, geo, uniforms, w, h } = windowMesh(
			i,
			map,
			drawn.radius / UNIT,
		);
		const pivot = new THREE.Group();
		const group = new THREE.Group();
		pivot.add(mesh);
		group.add(pivot);
		world.add(group);
		const { refl, reflUniforms } = createReflection(geo, uniforms);
		return {
			i,
			w,
			h,
			rim: WINDOW_RIM,
			mesh,
			pivot,
			group,
			uniforms,
			reflUniforms,
			refl,
			face: drawn.canvas,
			region: arrivalRegion(spec),
			state: "idle",
			shown: "idle",
			changed: Number.NEGATIVE_INFINITY,
			sel: 0,
			flash: 0,
			hover: 0,
			focus: 0,
		};
	}

	const windows = data.apps.map((_, i) => createWindow(i));
	const MIRROR = new THREE.Matrix4().makeScale(1, -1, 1);
	let arc = arcWindows(false);

	// On a phone the arc turns under a fixed camera so the selected app is always
	// overhead and its threads drop straight into the board; on wide screens the
	// camera turns instead and the arc stays put.
	const poses: WindowPose[] = Array.from({ length: 8 }, () => ({
		x: 0,
		z: 0,
		rotY: 0,
	}));
	function turn(arcYaw: number) {
		const c = Math.cos(arcYaw);
		const s = Math.sin(arcYaw);
		for (const [i, lw] of arc.entries()) {
			const z = lw.z - ARC_RADIUS;
			poses[i].x = lw.x * c - z * s;
			poses[i].z = ARC_RADIUS + lw.x * s + z * c;
			poses[i].rotY = lw.rotY - arcYaw;
		}
	}

	function place() {
		for (const win of windows) {
			const lw = arc[win.i];
			const pose = poses[win.i];
			win.group.position.set(pose.x, lw.hover, pose.z);
			win.group.rotation.set(0, pose.rotY, 0);
			win.pivot.rotation.set(-lw.lean, 0, 0);
			win.mesh.position.set(0, win.h / 2 + win.rim, 0);
		}
	}

	// Faces are drawn into their own canvas again and uploaded once, only when
	// the state a window shows changes or the page fonts arrive.
	function paint(win: WindowRec) {
		const drawn = drawAppWindow(
			data.apps[win.i],
			APP_SPECS[win.i],
			undefined,
			faceOf(win.shown),
			win.face,
		);
		swapImage(win.uniforms.uMap.value, drawn.canvas);
	}

	// Work leaving or arriving flashes the edge; returning to READY does not.
	function show(win: WindowRec, state: WindowState) {
		if (win.shown === state) return;
		win.shown = state;
		win.changed = state === "idle" ? Number.NEGATIVE_INFINITY : stage.t;
		paint(win);
	}

	function setState(app: number, state: WindowState) {
		const win = windows[app];
		if (win) win.state = state;
	}

	const raycaster = new THREE.Raycaster();
	const ndc = new THREE.Vector2();
	const windowMeshes = windows.map((w) => w.mesh);
	const hits: THREE.Intersection[] = [];
	function pick(clientX: number, clientY: number) {
		const rect = canvas.getBoundingClientRect();
		ndc.set(
			((clientX - rect.left) / rect.width) * 2 - 1,
			-((clientY - rect.top) / rect.height) * 2 + 1,
		);
		raycaster.setFromCamera(ndc, camera);
		hits.length = 0;
		raycaster.intersectObjects(windowMeshes, false, hits);
		return hits.length ? (hits[0].object.userData.app as number) : -1;
	}
	const onMove = (e: PointerEvent) => {
		const rect = canvas.getBoundingClientRect();
		stage.pointer.set(
			((e.clientX - rect.left) / rect.width) * 2 - 1,
			((e.clientY - rect.top) / rect.height) * 2 - 1,
		);
		stage.pointerInside = true;
		const h = pick(e.clientX, e.clientY);
		if (h !== stage.hover) {
			stage.hover = h;
			canvas.style.cursor = h >= 0 ? "pointer" : "";
		}
	};
	const onLeave = () => {
		stage.pointerInside = false;
		stage.hover = -1;
		canvas.style.cursor = "";
	};
	const onClick = (e: MouseEvent) => {
		const h = pick(e.clientX, e.clientY);
		if (h >= 0) options.onSelectApp?.(h);
	};
	canvas.addEventListener("pointermove", onMove);
	canvas.addEventListener("pointerleave", onLeave);
	canvas.addEventListener("click", onClick);

	function placeWindow(win: WindowRec, lw: WindowLayout, f: Frame) {
		const stagger =
			(stage.intro - 0.08 - Math.abs(lw.slot - 3.5) * 0.07) / 0.55;
		const rise = easeOut(clamp(stagger));
		const bob = Math.sin(f.t * 0.55 + win.i * 1.7) * 0.026 * f.motion;
		const pose = poses[win.i];
		win.group.position.set(
			pose.x,
			lw.hover +
				bob +
				win.sel * 0.1 +
				Math.max(win.hover, win.focus) * 0.05 -
				(1 - rise) * (win.h + 0.55),
			pose.z,
		);
		win.group.rotation.set(
			0,
			pose.rotY,
			Math.sin(f.t * 0.4 + win.i) * 0.004 * f.motion,
		);
		win.pivot.rotation.x = -lw.lean * (0.35 + 0.65 * rise);
	}

	// The region an arrival changed glows for a moment; the still holds it.
	function glowAmount(win: WindowRec, f: Frame) {
		if (win.shown !== "arrived" || !win.region) return 0;
		if (!f.motion) return STILL_GLOW;
		const since = f.t - win.changed;
		if (since >= REGION_S) return 0;
		return smooth(0, 0.18, since) * (1 - smooth(0.7, REGION_S, since));
	}

	function highlight(win: WindowRec, f: Frame) {
		const u = win.uniforms;
		const amount = glowAmount(win, f);
		u.uHLAmt.value = amount * f.reveal.sky;
		if (amount > 0 && win.region) {
			u.uHL.value.set(
				win.region[0],
				win.region[1],
				win.region[2],
				win.region[3],
			);
		}
	}

	function shadeWindow(win: WindowRec, f: Frame) {
		const u = win.uniforms;
		const dark = f.reveal.sky;
		u.uSel.value = win.sel;
		u.uFlash.value = win.flash * lerp(0.25, 1, dark);
		u.uHover.value = win.hover;
		u.uFocus.value = win.focus;
		u.uDark.value = dark;
		highlight(win, f);
	}

	function reflectWindow(win: WindowRec, f: Frame) {
		win.group.updateMatrixWorld(true);
		win.refl.matrixWorld.multiplyMatrices(MIRROR, win.mesh.matrixWorld);
		win.refl.visible = f.reveal.reflections > 0.01;
		win.reflUniforms.uMirrorFade.value = f.reveal.reflections * 0.46;
	}

	// The scenario's cue (work leaving or arriving) and this window's own
	// state change; the still holds a steady glow on every touched window.
	function flashOf(
		win: WindowRec,
		f: Frame,
		cues: WindowCues,
		involved: boolean,
	) {
		if (!f.motion) return decay(win.flash, involved ? 0.45 : 0, f.dt, 2.2);
		const cue = cues.flash.get(win.i) ?? 0;
		const arrival = pulse(f.t - win.changed, FLASH_S) ** 1.5;
		return Math.max(decay(win.flash, cue, f.dt, 2.2), arrival);
	}

	function updateWindow(win: WindowRec, f: Frame, cues: WindowCues) {
		show(win, f.alive ? win.state : "idle");
		const lw = arc[win.i];
		const involved = cues.apps.includes(win.i);
		win.sel = lerp(win.sel, stage.app === win.i ? 1 : 0, f.k(6));
		win.flash = flashOf(win, f, cues, involved);
		win.hover = lerp(win.hover, stage.hover === win.i ? 1 : 0, f.k(10));
		win.focus = lerp(win.focus, stage.focus === win.i ? 1 : 0, f.k(12));
		placeWindow(win, lw, f);
		shadeWindow(win, f);
		reflectWindow(win, f);
		const pose = poses[win.i];
		shared.uWinA.value[win.i].set(pose.x, pose.z, lw.w / 2, pose.rotY);
		shared.uWinB.value[win.i].set(
			(win.h + win.rim * 2) * Math.sin(lw.lean),
			Math.max(0, win.group.position.y),
			win.sel * 0.8 + (involved ? 0.35 * f.reveal.sky : 0),
			win.flash * f.reveal.sky,
		);
	}

	// The still shows every window the scenario delivers to in its end state;
	// windows that only send work read live.
	function settleStill(cues: WindowCues) {
		for (const win of windows) {
			const touched = cues.apps.includes(win.i);
			const ends = cues.arrivals.includes(win.i);
			setState(win.i, ends ? "arrived" : touched ? "live" : "idle");
		}
	}

	return {
		count: windows.length,
		width: (app) => arc[app].w,
		theta: (app) => arc[app].theta,
		pose: (app) => poses[app],
		dock: (app, offset = 0, out = new THREE.Vector3()) =>
			windowDock(poses[app], offset, out),
		corner: (app, i, out) => windowCorner(app, arc[app].w, poses[app], i, out),
		layout(portrait) {
			arc = arcWindows(portrait);
		},
		turn,
		place,
		update(frame, cues) {
			if (!frame.motion) settleStill(cues);
			for (const win of windows) updateWindow(win, frame, cues);
		},
		setWindowState: setState,
		redraw() {
			for (const win of windows) paint(win);
		},
		dispose() {
			canvas.removeEventListener("pointermove", onMove);
			canvas.removeEventListener("pointerleave", onLeave);
			canvas.removeEventListener("click", onClick);
			res.dispose();
		},
	};
}
