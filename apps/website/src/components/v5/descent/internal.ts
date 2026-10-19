import type * as THREE from "three";
import type { DescentData, DescentOptions, DescentSafeArea } from "./types";

export type Vec3Tuple = readonly [number, number, number];
export type LayoutKind = "portrait" | "landscape";
export type Uniform<T> = { value: T };

/** Frame-rate independent easing factor for a rate per second; 1 when dt ≤ 0. */
export type Rate = (rate: number) => number;

export interface FogUniforms {
	uFogColor: Uniform<THREE.Color>;
	uFogDensity: Uniform<number>;
}

/** Uniform objects the modules' materials share by reference. */
export interface SharedUniforms extends FogUniforms {
	uTime: Uniform<number>;
	uMotion: Uniform<number>;
	/** Per app window: x, z, half width, yaw. Written by the windows. */
	uWinA: Uniform<THREE.Vector4[]>;
	/** Per app window: lean depth, height, energy, flash. Written by the windows. */
	uWinB: Uniform<THREE.Vector4[]>;
}

/** Reveal amounts derived from the scroll progress through the shared TIMELINE, each 0–1. */
export interface Reveal {
	/** The floor has turned to dark glass. */
	glass: number;
	/** The stage background has turned dark. */
	sky: number;
	/** Where the sky's dark front is, from the horizon (0) to past the frame's top (1); it speeds up as it rises. */
	skyFront: number;
	/** What is left of the window reflections. */
	reflections: number;
	/** The board has faded up beneath the glass. */
	board: number;
	/** Progress through the threads' draw-in (linear; each thread eases its own share). */
	threads: number;
	/** The active route has lit up. */
	lit: number;
	/** The copy band has faded in. */
	band: number;
}

/** Live scene state. scene.ts writes it unless a field names another writer. */
export interface Stage {
	/** Scene clock, seconds. */
	t: number;
	frozen: boolean;
	progress: number;
	progressTarget: number;
	intro: number;
	introTarget: number;
	/** Selected app. */
	app: number;
	scenario: number;
	scenarioStart: number;
	/** Hovered app, or -1. Written by the windows. */
	hover: number;
	/** App whose window shows keyboard focus, or -1. */
	focus: number;
	/** Pointer in NDC, y down. Written by the windows. */
	pointer: THREE.Vector2;
	/** Written by the windows. */
	pointerInside: boolean;
	pointerSmooth: THREE.Vector2;
	/** Camera yaw around the arc (wide screens). Written by the camera rig. */
	yaw: number;
	/** Yaw of the window arc (phones turn the arc instead). Written by the camera rig. */
	arcYaw: number;
	width: number;
	height: number;
	dpr: number;
	portrait: boolean;
	/** Insets from setSafeArea; null frames the picture without them. */
	safeArea: DescentSafeArea | null;
	disposed: boolean;
	/** 0 under reduced motion, else 1. */
	readonly motion: number;
}

/** Per-frame values, computed once by scene.ts after the camera has moved. */
export interface Frame {
	/** Seconds since the last frame; 0 snaps every easing. */
	readonly dt: number;
	readonly k: Rate;
	readonly t: number;
	readonly motion: number;
	readonly reveal: Reveal;
	/** The layer has come alive (TIMELINE.alive): windows show the scenario's states. */
	readonly alive: boolean;
}

/** The scenario playing beneath. */
export interface ScenarioClock {
	/** Index into data.scenarios. */
	readonly index: number;
	/** Seconds since setScenario, setScenarioClock or replayScenario. */
	readonly elapsed: number;
}

/** What the scenario beneath asks of the windows this frame. */
export interface WindowCues {
	/** Apps the scenario touches. */
	readonly apps: readonly number[];
	/**
	 * Apps the scenario delivers work to. The reduced-motion still shows their
	 * end state; the other apps it touches only send work, so they read live.
	 */
	readonly arrivals: readonly number[];
	/** Edge flash per app while work leaves or reaches it, 0–1; a missing app is 0. */
	readonly flash: ReadonlyMap<number, number>;
}

export type WindowState = "idle" | "live" | "arrived";

/** A window's place after the arc turned. */
export interface WindowPose {
	x: number;
	z: number;
	rotY: number;
}

/** Read access to the windows, for the camera and everything beneath. */
export interface WindowsView {
	readonly count: number;
	/** Width of app `app`'s window in world units. */
	width(app: number): number;
	/** Angle of app `app` on the arc, radians from the centre. */
	theta(app: number): number;
	pose(app: number): Readonly<WindowPose>;
	/** Point on the floor under app `app`'s dock, `offset` along the dock; written into `out` when given. */
	dock(app: number, offset?: number, out?: THREE.Vector3): THREE.Vector3;
	/** Corner `i` of app `app`'s window at rest (0, 1 its base, 2, 3 its leaning top, left to right), into `out`. */
	corner(app: number, i: number, out: THREE.Vector3): THREE.Vector3;
}

export interface Windows extends WindowsView {
	/** Lays the windows out on the arc. */
	layout(portrait: boolean): void;
	/** Poses every window on the arc turned by `arcYaw`. */
	turn(arcYaw: number): void;
	/** Puts the window groups at their poses after a layout. */
	place(): void;
	/** Selection, hover, flash, rise, shading, reflections and the shared window uniforms. */
	update(frame: Frame, cues: WindowCues): void;
	/**
	 * Switches app `app`'s window to a state: "live" flips its READY badge to
	 * LIVE, "arrived" also draws its end state. A change redraws that one face
	 * and flashes its edge; setting the current state again does nothing.
	 */
	setWindowState(app: number, state: WindowState): void;
	/** Repaints the window faces (fonts loaded, context restored). */
	redraw(): void;
	dispose(): void;
}

export interface Surface {
	/**
	 * Sky, fog and the floor at the reveal, for the camera as placed this
	 * frame; returns how dark (0–1) the sky is under the header's bottom edge.
	 */
	update(reveal: Reveal): number;
	dispose(): void;
}

/** Where the board lies: world units per board unit, the world point of board (0, 0) and its lean. */
export interface BoardPlacement {
	readonly unit: number;
	readonly x: number;
	readonly y: number;
	readonly z: number;
	/** Radians the board leans back from upright, so its top row recedes. */
	readonly tilt: number;
}

/** The board as the composition sees it, in board units. */
export interface BoardFrame {
	/** Everything drawn on the board. */
	readonly content: { x: number; y: number; width: number; height: number };
	/** Board x the picture centres under the windows. */
	readonly centre: number;
	/** Width the picture must show: the whole board, or a phone's columns. */
	readonly span: number;
}

/** A correction applied on top of the key timeline. */
export interface Framing {
	/** World-space pan added to the camera and its target. */
	readonly offset: Vec3Tuple;
	/** tan(fov / 2) is divided by it: below 1 widens the view. */
	readonly zoom: number;
	/** Vertical view offset in CSS px; positive moves the picture up. */
	readonly shiftPx: number;
	/** CSS px from the stage's top at which this framing shows the arc's reference point (ARC_REFERENCE). */
	readonly ref: number;
}

/** The final picture for the stage's size, insets and layout. */
export interface Composition {
	/** The rest framing: the windows between the sub-head and the dock. */
	readonly rest: Framing;
	/** The final picture before the copy band shows: lowered a little into the band's room. */
	readonly open: Framing;
	/** The final framing with the copy band: windows, threads and board above it. */
	readonly band: Framing;
	/** Where the board lies; on phones x is where the centred columns sit. */
	readonly board: BoardPlacement;
	/** CSS px of the board's widest row and of the stage, for checks and tests. */
	readonly boardWidthPx: number;
}

export interface Composer {
	/** The composition for the current size, insets, layout and board; recomputed only when one changes. */
	get(): Composition;
	/** The board was laid out again (fonts): the next get() recomputes. */
	invalidate(): void;
}

export interface Beneath {
	/** Lays everything beneath out; the windows are already laid out and turned. */
	layout(portrait: boolean): void;
	/** Re-routes the app threads if the arc turned since they were last routed. */
	followWindows(): void;
	/** Window cues for this frame. Pure; scene.ts reads them before the windows update. */
	cues(scenario: ScenarioClock): WindowCues;
	/** Advances everything beneath by one frame; runs after the windows update. */
	update(frame: Frame, scenario: ScenarioClock): void;
	/** Repaints the canvas faces once the page fonts are ready. */
	redraw(): void;
	/** The board as the composition frames it. */
	frame(): BoardFrame;
	/** Calls `cb` when work reaches app `app`; returns an unsubscribe. */
	onArrive(cb: (app: number) => void): () => void;
	dispose(): void;
}

export interface CameraKey {
	p: number;
	pos: Vec3Tuple;
	at: Vec3Tuple;
	fov: number;
}

export interface CameraRig {
	/** Camera yaw that turns the selected app into view (wide screens). */
	yawTarget(p: number): number;
	/** Arc yaw that brings the app with keyboard focus, else the selected app, overhead (phones). */
	arcYawTarget(): number;
	/** Eases stage.yaw and stage.arcYaw toward their targets. */
	steer(p: number, k: Rate): void;
	/** Positions the camera on the key timeline, framed by the composition. */
	aim(p: number, reveal: Reveal): void;
	/** Calls onTheme on the frame the picture under the header turns dark or light (`underHeader`: 0–1). */
	publishTheme(underHeader: number): void;
}

export interface Post {
	/** A composer exists (after the first resize). */
	readonly active: boolean;
	/**
	 * Resizes the composer in place at the render's pixel ratio `dpr`; rebuilds it
	 * only when the MSAA sample count, chosen by the device's `deviceDpr`, changes.
	 */
	resize(width: number, height: number, dpr: number, deviceDpr: number): void;
	/** Compiles every pass's program without blocking, each in the state it draws in. */
	compile(): Promise<void>;
	update(frame: Frame): void;
	render(): void;
	/** Renders once with every pass enabled, so their buffers and targets exist ahead of use. */
	warm(): void;
	/** Frees the render targets while nothing is drawn; the next render allocates them again. */
	release(): void;
	dispose(): void;
}

/** GPU resources a module creates and disposes together. */
export interface Resources {
	track<T extends { dispose(): void }>(resource: T): T;
	material(params: THREE.ShaderMaterialParameters): THREE.ShaderMaterial;
	/** sRGB, mipmapped, anisotropic. */
	canvasTexture(canvas: HTMLCanvasElement): THREE.CanvasTexture;
	dispose(): void;
}

/** Uniforms of the window panel shader. */
export interface PanelUniforms extends FogUniforms {
	[name: string]: THREE.IUniform;
	uMap: Uniform<THREE.CanvasTexture>;
	uSize: Uniform<THREE.Vector2>;
	uRadius: Uniform<number>;
	uKind: Uniform<number>;
	/** 0 on the porcelain, 1 on the dark stage. */
	uDark: Uniform<number>;
	uSel: Uniform<number>;
	uFlash: Uniform<number>;
	uHover: Uniform<number>;
	/** Keyboard focus on the window's control: a wide ring inside its edge. */
	uFocus: Uniform<number>;
	uMirror: Uniform<number>;
	uMirrorFade: Uniform<number>;
	uHL: Uniform<THREE.Vector4>;
	uHLAmt: Uniform<number>;
}

/** What every module receives from scene.ts. */
export interface SceneContext {
	readonly container: HTMLElement;
	readonly canvas: HTMLCanvasElement;
	readonly renderer: THREE.WebGLRenderer;
	readonly scene: THREE.Scene;
	readonly camera: THREE.PerspectiveCamera;
	/** Parent of the windows above the floor. */
	readonly world: THREE.Group;
	/** Parent of everything beneath the floor. */
	readonly under: THREE.Group;
	readonly data: DescentData;
	readonly options: DescentOptions;
	readonly stage: Stage;
	readonly shared: SharedUniforms;
	/** Anisotropy for canvas textures (device maximum, capped at 8). */
	readonly maxAniso: number;
}
