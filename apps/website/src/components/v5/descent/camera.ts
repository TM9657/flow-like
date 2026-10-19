import * as THREE from "three";
import {
	LENS,
	cameraKeys,
	keyFov,
	sampleKeysInto,
	zoomFov,
} from "./camera-keys";
import { arcReference } from "./composition";
import type {
	CameraRig,
	Composer,
	Composition,
	LayoutKind,
	Reveal,
	SceneContext,
	WindowsView,
} from "./internal";
import { ARC_RADIUS, TIMELINE, clamp, easeOut, lerp, smooth } from "./shared";

/** The picture under the header reads dark once the sky there is past this mix. */
const DARK_AT = 0.5;
/** Wide screens turn toward the selected app at rest, until the pull-back. */
const YAW = { comfort: 0.2, until: [0.06, 0.3] } as const;
/** A slow sway once the layer is alive, world units; and the pointer's parallax. */
const SWAY = 0.1;
const PARALLAX = { x: 0.3, y: 0.14 } as const;
/**
 * Progress over which the arc rises from its place under the sub-head to its
 * final place, as the sub-head leaves; the intro keeps the rest framing.
 */
const RISE = [0.03, 0.3] as const;
const PIN_FROM = [0, 0.03] as const;

function yawFor(p: number, portrait: boolean, theta: number) {
	if (portrait) return 0;
	const y =
		Math.abs(theta) > YAW.comfort ? theta - Math.sign(theta) * YAW.comfort : 0;
	return y * (1 - smooth(YAW.until[0], YAW.until[1], p));
}

function rotateAboutArc(v: THREE.Vector3, yaw: number, cz: number) {
	const c = Math.cos(yaw);
	const s = Math.sin(yaw);
	const x = v.x;
	const z = v.z - cz;
	v.x = x * c - z * s;
	v.z = cz + x * s + z * c;
}

export function createCameraRig(
	ctx: SceneContext,
	windows: WindowsView,
	composer: Composer,
): CameraRig {
	const { camera, stage, options } = ctx;
	const camPos = new THREE.Vector3();
	const camAt = new THREE.Vector3();
	const ref = new THREE.Vector3();
	const tmp: number[] = [0, 0, 0];
	let dark = false;

	const kind = (): LayoutKind => (stage.portrait ? "portrait" : "landscape");
	const yawTarget = (p: number) =>
		yawFor(p, stage.portrait, windows.theta(stage.app));
	const arcYawTarget = () => (stage.portrait ? -windows.theta(stage.app) : 0);

	/** The key camera at p with the intro's lift, the rest yaw and the arc's pan, into camPos and camAt. */
	function keyPose(p: number, pan: number) {
		const keys = cameraKeys[kind()];
		const { intro } = LENS[kind()];
		const lift = 1 - easeOut(clamp(stage.intro));
		sampleKeysInto(keys, p, "pos", tmp);
		camPos.set(
			tmp[0] + intro[0] * lift,
			tmp[1] + intro[1] * lift,
			tmp[2] + intro[2] * lift,
		);
		sampleKeysInto(keys, p, "at", tmp);
		camAt.set(tmp[0], tmp[1], tmp[2]);
		rotateAboutArc(camPos, stage.yaw, ARC_RADIUS);
		rotateAboutArc(camAt, stage.yaw, ARC_RADIUS);
		camPos.x += pan;
		camAt.x += pan;
	}

	/** The key's field of view at p, zoomed from the rest framing to the final one; returns it. */
	function lens(p: number, reveal: Reveal, comp: Composition, final: number) {
		const { rest, open, band } = comp;
		const zoom = lerp(
			lerp(rest.zoom, open.zoom, final),
			band.zoom,
			reveal.band,
		);
		const aspect = stage.width / Math.max(1, stage.height);
		const fov = zoomFov(keyFov(kind(), p, aspect), zoom);
		if (Math.abs(camera.fov - fov) > 1e-4) {
			camera.fov = fov;
			camera.updateProjectionMatrix();
		}
		return fov;
	}

	/** The lens shift (CSS px) that puts the arc's reference point on its path, for the key camera. */
	function pin(
		p: number,
		reveal: Reveal,
		comp: Composition,
		pan: number,
		fov: number,
	) {
		camera.position.copy(camPos);
		camera.lookAt(camAt);
		camera.updateMatrixWorld();
		arcReference(kind(), pan, ref).applyMatrix4(camera.matrixWorldInverse);
		const half = stage.height / 2;
		const tan = Math.tan(THREE.MathUtils.degToRad(fov) / 2);
		const shown = half - (ref.y / Math.max(1e-3, -ref.z)) * (half / tan);
		const { rest, open, band } = comp;
		const rise = smooth(RISE[0], RISE[1], p);
		const wanted = lerp(lerp(rest.ref, open.ref, rise), band.ref, reveal.band);
		const pinned = smooth(PIN_FROM[0], PIN_FROM[1], p);
		return lerp(rest.shiftPx, shown - wanted, pinned);
	}

	/** A slow sway once the layer is alive, and the pointer's parallax. */
	function drift(p: number) {
		const motion = stage.motion;
		const sway = smooth(TIMELINE.glass[1], TIMELINE.alive + 0.1, p) * motion;
		const par = stage.pointerSmooth;
		camPos.x += Math.sin(stage.t * 0.13) * SWAY * sway;
		camPos.y += Math.sin(stage.t * 0.09 + 1.3) * SWAY * 0.35 * sway;
		camPos.x += par.x * PARALLAX.x * motion;
		camPos.y -= par.y * PARALLAX.y * motion;
	}

	function viewOffset(shiftPx: number) {
		const { width, height } = stage;
		if (Math.abs(shiftPx) > 0.5) {
			camera.setViewOffset(width, height, 0, shiftPx, width, height);
		} else if (camera.view?.enabled) camera.clearViewOffset();
	}

	return {
		yawTarget,
		arcYawTarget,
		steer(p, k) {
			stage.yaw = lerp(stage.yaw, yawTarget(p), k(3.2));
			stage.arcYaw = lerp(stage.arcYaw, arcYawTarget(), k(3.2));
		},
		// The camera follows its keys; the lens keeps the arc's reference point
		// on a smooth path from its rest place (under the sub-head) to its final
		// place, so the windows rise as the sub-head leaves and never reach the
		// header, and the picture settles up as the copy band arrives.
		aim(p, reveal: Reveal) {
			const comp = composer.get();
			const final = smooth(TIMELINE.glass[0], TIMELINE.glass[1], p);
			const pan = comp.open.offset[0] * final;
			keyPose(p, pan);
			const fov = lens(p, reveal, comp, final);
			const shiftPx = pin(p, reveal, comp, pan, fov);
			drift(p);
			camera.position.copy(camPos);
			camera.lookAt(camAt);
			viewOffset(shiftPx);
			camera.updateMatrixWorld();
		},
		// The sky is what shows under the header, so the page switches its
		// header and controls on the frame the sky there turns.
		publishTheme(underHeader) {
			const next = underHeader >= DARK_AT;
			if (next === dark) return;
			dark = next;
			options.onTheme?.(dark ? "dark" : "light");
		},
	};
}
