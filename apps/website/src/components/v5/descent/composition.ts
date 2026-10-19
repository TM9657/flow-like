import * as THREE from "three";
import { cameraKeys, keyFov, sampleKeysInto } from "./camera-keys";
import type {
	BoardFrame,
	BoardPlacement,
	Composer,
	Composition,
	Framing,
	LayoutKind,
	Stage,
	WindowsView,
} from "./internal";
import { clamp } from "./shared";
import type { DescentInsets, DescentSafeArea } from "./types";
import { APP_SPECS } from "./ui-draw";
import { WINDOW_HOVER, WINDOW_LEAN, windowTop } from "./windows";

/** CSS px kept between the picture and the safe-area edges. */
export const MARGIN_Y = 14;
const MARGIN_X = { min: 14, share: 0.028, max: 40 } as const;

/** Side margin of the picture at a viewport width, in CSS px. */
export const marginX = (width: number) =>
	clamp(width * MARGIN_X.share, MARGIN_X.min, MARGIN_X.max);

/** World units the top corners keep above the window tops: the selected window rises. */
const HEADROOM = 0.14;
/**
 * How far the picture may sit lower before the copy band shows, centred in the
 * room the band will take (a share of the stage's height, at most `max` CSS
 * px); it settles up by that much as the band arrives. The band only shows
 * once the picture has covered most of the settle (controller BAND_SHOW), so
 * the two never meet on screen.
 */
export const SETTLE = { share: 0.17, max: 160 } as const;
export const settleMax = (height: number) =>
	Math.min(SETTLE.max, height * SETTLE.share);

interface Range {
	readonly min: number;
	readonly best: number;
	readonly max: number;
}

/**
 * How the final picture may be proportioned. `arc` is the share of the stage's
 * width the windows take (phones: the selected window alone), `board` the share
 * the board's widest row takes (phones: the columns in view), `tilt` how far the
 * board leans back (radians), `gap` the CSS px of thread between the lowest dock
 * and the board, as a share of the stage height. The solver keeps every value
 * as close to its best as the room between the header and the copy allows.
 */
export const PROPORTIONS: Record<
	LayoutKind,
	{ arc: Range; board: Range; tilt: Range; gap: Range }
> = {
	landscape: {
		arc: { min: 0.7, best: 0.8, max: 0.86 },
		board: { min: 0.52, best: 0.62, max: 0.66 },
		tilt: { min: 0.42, best: 0.62, max: 1.05 },
		gap: { min: 0.075, best: 0.11, max: 0.16 },
	},
	portrait: {
		arc: { min: 0.5, best: 0.64, max: 0.7 },
		board: { min: 0.78, best: 0.88, max: 0.92 },
		tilt: { min: 0.35, best: 0.55, max: 1.1 },
		gap: { min: 0.05, best: 0.085, max: 0.12 },
	},
};

/** The board's top row lies this much deeper than the centre dock, along the view. */
const BOARD_DEPTH = 1.12;
/** The board's top row stays at least this far beneath the floor (world units). */
const BENEATH = 0.5;
/** A phone frames a window of this size (world units) as the selected one, whichever app it is. */
const PHONE_WINDOW = { width: 3, top: 2.45 } as const;
/** Cost of a CSS px of empty band left between the board and the copy. */
const SPARE_COST = 1 / 45;

/** Vertical extent of the window arc on screen, as NDC y (up) of its top and its base. */
export interface ArcExtent {
	top: number;
	base: number;
}

/**
 * Zoom and lens shift that put the arc between the band's insets: phones pull
 * the arc up under the sub-head, wide screens only move it when it would touch
 * the copy above or the dock below; a band too short for it zooms out.
 */
export function fitArc(
	extent: ArcExtent,
	insets: DescentInsets,
	height: number,
	pullUp: boolean,
): { zoom: number; shiftPx: number } {
	const bandTop = 1 - (2 * insets.top) / height;
	const bandBottom = -1 + (2 * insets.bottom) / height;
	const tall = extent.top - extent.base;
	const room = bandTop - bandBottom;
	if (room <= 0 || tall <= 0) return { zoom: 1, shiftPx: 0 };
	if (tall > room) {
		const zoom = room / tall;
		return { zoom, shiftPx: ((bandTop - extent.top * zoom) * height) / 2 };
	}
	const lo = bandBottom - extent.base;
	const hi = bandTop - extent.top;
	const shift = pullUp ? hi : clamp(0, lo, hi);
	return { zoom: 1, shiftPx: (shift * height) / 2 };
}

export interface ComposeInput {
	readonly width: number;
	readonly height: number;
	readonly area: DescentSafeArea | null;
	readonly kind: LayoutKind;
	readonly windows: WindowsView;
	readonly board: BoardFrame;
}

/** Height of the tallest window top the arc shows (phones show the phone app too). */
const tallest = (kind: LayoutKind) =>
	APP_SPECS.reduce(
		(best, spec, i) =>
			spec.phone && kind === "landscape" ? best : Math.max(best, windowTop(i)),
		0,
	);

/** A camera looking from `eye` at `target`, with its basis and depth helpers. */
class View {
	readonly eye = new THREE.Vector3();
	readonly forward = new THREE.Vector3();
	readonly right = new THREE.Vector3();
	readonly up = new THREE.Vector3();
	private readonly rel = new THREE.Vector3();

	aim(eye: THREE.Vector3, target: THREE.Vector3) {
		this.eye.copy(eye);
		this.forward.subVectors(target, eye).normalize();
		this.right
			.crossVectors(this.forward, THREE.Object3D.DEFAULT_UP)
			.normalize();
		this.up.crossVectors(this.right, this.forward);
	}
	depth(p: THREE.Vector3) {
		return this.rel.subVectors(p, this.eye).dot(this.forward);
	}
	/** Tangent coordinates (x right, y up) of a world point. */
	tan(p: THREE.Vector3, out: { x: number; y: number }) {
		this.rel.subVectors(p, this.eye);
		const d = Math.max(1e-4, this.rel.dot(this.forward));
		out.x = this.rel.dot(this.right) / d;
		out.y = this.rel.dot(this.up) / d;
		return out;
	}
	/** The world point at tangent (tx, ty) and `depth` along the view. */
	at(tx: number, ty: number, depth: number, out: THREE.Vector3) {
		return out
			.copy(this.eye)
			.addScaledVector(this.forward, depth)
			.addScaledVector(this.right, tx * depth)
			.addScaledVector(this.up, ty * depth);
	}
}

interface ArcShape {
	/** Tangent half width of the arc about the view's centre line. */
	half: number;
	/** Tangent y of the arc's top and of its lowest dock. */
	top: number;
	dockLow: number;
	/** Depth of the dock nearest the centre line. */
	dockDepth: number;
}

interface BoardFit {
	placement: BoardPlacement;
	/** Tangent y of the board's lowest drawn point. */
	bottom: number;
	/** Tangent width of its widest row. */
	width: number;
}

const tmp = new THREE.Vector3();
const tmpB = new THREE.Vector3();
const corner = new THREE.Vector3();
const boardTop = new THREE.Vector3();
const boardOrigin = new THREE.Vector3();
const t2 = { x: 0, y: 0 };
const ex = new THREE.Vector3();
const ey = new THREE.Vector3();

/** The arc as seen from `view`: wide screens take every window, phones the canonical selected one. */
function arcShape(input: ComposeInput, view: View) {
	const shape: ArcShape = {
		half: 0,
		top: Number.NEGATIVE_INFINITY,
		dockLow: Number.POSITIVE_INFINITY,
		dockDepth: 1,
	};
	const take = (p: THREE.Vector3, top: boolean) => {
		view.tan(p, t2);
		shape.half = Math.max(shape.half, Math.abs(t2.x));
		if (top) shape.top = Math.max(shape.top, t2.y);
	};
	const dock = (p: THREE.Vector3) => {
		view.tan(p, t2);
		shape.dockLow = Math.min(shape.dockLow, t2.y);
	};
	if (input.kind === "portrait") {
		const half = PHONE_WINDOW.width / 2;
		for (const x of [-half, half]) {
			take(corner.set(x, PHONE_WINDOW.top + HEADROOM, -0.5), true);
			take(corner.set(x, 0, 0), false);
		}
		dock(corner.set(0, 0, 0));
		shape.dockDepth = view.depth(corner);
		return shape;
	}
	const { windows } = input;
	let nearest = Number.POSITIVE_INFINITY;
	for (let app = 0; app < windows.count; app++) {
		for (let i = 0; i < 4; i++) {
			windows.corner(app, i, corner);
			if (i >= 2) corner.y += HEADROOM;
			take(corner, i >= 2);
		}
		const d = windows.dock(app, 0, corner);
		dock(d);
		view.tan(d, t2);
		if (Math.abs(t2.x) < nearest) {
			nearest = Math.abs(t2.x);
			shape.dockDepth = view.depth(d);
		}
	}
	return shape;
}

/**
 * The board leaning back by `tilt` with its top row centred at tangent
 * (0, `topY`), scaled so its widest row spans `width` in tangent units.
 */
function fitBoard(
	input: ComposeInput,
	view: View,
	shape: ArcShape,
	topY: number,
	width: number,
	tilt: number,
): BoardFit {
	const { content, centre, span } = input.board;
	ex.set(1, 0, 0);
	ey.set(0, Math.cos(tilt), -Math.sin(tilt));
	let depth = shape.dockDepth * BOARD_DEPTH;
	view.at(0, topY, depth, tmp);
	if (tmp.y > -BENEATH) {
		// Too shallow to lie under the floor: the same screen point, deeper.
		const fall = view.eye.y + BENEATH;
		const down = -(view.forward.y + topY * view.up.y);
		if (down > 1e-4) depth = Math.max(depth, fall / down);
		view.at(0, topY, depth, tmp);
	}
	const top = boardTop.copy(tmp);
	let unit = 0.02;
	let widest = width;
	let bottom = topY;
	const point = (bx: number, by: number, out: THREE.Vector3) =>
		out
			.copy(top)
			.addScaledVector(ex, (bx - centre) * unit)
			.addScaledVector(ey, -(by - content.y) * unit);
	for (let i = 0; i < 6; i++) {
		const rows = [content.y, content.y + content.height];
		widest = 0;
		bottom = Number.POSITIVE_INFINITY;
		for (const by of rows) {
			const l = view.tan(point(centre - span / 2, by, tmpB), t2).x;
			const r = view.tan(point(centre + span / 2, by, tmpB), t2).x;
			widest = Math.max(widest, r - l);
			bottom = Math.min(bottom, t2.y);
		}
		unit *= width / Math.max(1e-6, widest);
	}
	const origin = point(0, 0, boardOrigin);
	return {
		placement: { unit, x: origin.x, y: origin.y, z: origin.z, tilt },
		bottom,
		width: widest,
	};
}

/** The final picture as solved: its zoom, the board, and its extent. */
interface Solved {
	zoom: number;
	board: BoardFit;
	/** Tangent y of the picture's top and bottom. */
	top: number;
	bottom: number;
	/** CSS px per tangent unit at `zoom`. */
	scale: number;
}

interface Candidate extends Solved {
	cost: number;
}

const devi = (v: number, r: Range, spread: number) =>
	((v - r.best) / spread) ** 2;

/**
 * The final composition: the arc spans its share of the width, the board
 * hangs `gap` px below the lowest dock, leaning back, its widest row at its
 * share; the solver picks the shares, the lean and the gap that fill the room
 * between the header and the air above the copy band best.
 */
function solve(input: ComposeInput, view: View, fov: number): Solved {
	const { width: w, height: h, kind } = input;
	const prop = PROPORTIONS[kind];
	const aspect = w / h;
	const insets = input.area?.beneath ?? { top: 0, bottom: 0 };
	const room = h - insets.top - insets.bottom - 2 * MARGIN_Y;
	const shape = arcShape(input, view);
	const baseTan = Math.tan(THREE.MathUtils.degToRad(fov) / 2);
	const across = 1 - (2 * marginX(w)) / w;
	let best: Candidate | null = null;
	for (let arc = prop.arc.max; arc >= prop.arc.min - 1e-9; arc -= 0.02) {
		// Tangent half height of the view once the arc spans its share.
		const tanV = shape.half / (arc * across * aspect);
		const px = h / (2 * tanV);
		for (
			let share = prop.board.max;
			share >= prop.board.min - 1e-9;
			share -= 0.02
		) {
			const boardTan = share * 2 * tanV * aspect;
			for (const gapShare of [prop.gap.max, prop.gap.best, prop.gap.min]) {
				const gap = gapShare * h;
				const topY = shape.dockLow - gap / px;
				for (
					let tilt = prop.tilt.min;
					tilt <= prop.tilt.max + 1e-9;
					tilt += 0.07
				) {
					const board = fitBoard(input, view, shape, topY, boardTan, tilt);
					const used = (shape.top - board.bottom) * px;
					const over = used - room;
					if (over > 0.5) continue;
					const spare = -over;
					const cost =
						devi(share, prop.board, 0.05) * 3 +
						devi(arc, prop.arc, 0.06) * 2 +
						devi(tilt, prop.tilt, 0.25) +
						devi(gapShare, prop.gap, 0.03) +
						(spare * SPARE_COST) ** 2;
					if (best && cost >= best.cost) continue;
					best = {
						cost,
						zoom: baseTan / tanV,
						board,
						top: shape.top,
						bottom: board.bottom,
						scale: px,
					};
				}
			}
		}
	}
	if (!best) {
		// Nothing fits (a tiny stage): the narrowest picture, scaled into the room.
		const tanV = shape.half / (prop.arc.min * across * aspect);
		const px0 = h / (2 * tanV);
		const topY = shape.dockLow - (prop.gap.min * h) / px0;
		const boardTan = prop.board.min * 2 * tanV * aspect;
		const board = fitBoard(input, view, shape, topY, boardTan, prop.tilt.max);
		const used = (shape.top - board.bottom) * px0;
		const shrink = Math.min(1, room / Math.max(1, used));
		best = {
			cost: 0,
			zoom: (baseTan / tanV) * shrink,
			board,
			top: shape.top,
			bottom: board.bottom,
			scale: px0 * shrink,
		};
	}
	return best;
}

/**
 * The point the camera keeps on a smooth path while it moves: the top of the
 * tallest window, leaning back, at the arc's centre line `x`, `headroom` above it.
 */
export function arcReference(
	kind: LayoutKind,
	x: number,
	out: THREE.Vector3,
	headroom = HEADROOM,
) {
	const top = tallest(kind);
	const back = (top - WINDOW_HOVER) * Math.tan(WINDOW_LEAN);
	return out.set(x, top + headroom, -back);
}

/** The camera x at which the arc's outer corners lie equally far from the frame's sides. */
function arcCentre(input: ComposeInput, eye: THREE.Vector3) {
	if (input.kind === "portrait") return 0;
	let lo = Number.POSITIVE_INFINITY;
	let hi = Number.NEGATIVE_INFINITY;
	let loX = 0;
	let hiX = 0;
	let loDepth = 1;
	let hiDepth = 1;
	for (let app = 0; app < input.windows.count; app++) {
		for (let i = 0; i < 4; i++) {
			const c = input.windows.corner(app, i, corner);
			const depth = Math.max(1e-3, eye.z - c.z);
			const t = (c.x - eye.x) / depth;
			if (t < lo) {
				lo = t;
				loX = c.x;
				loDepth = depth;
			}
			if (t > hi) {
				hi = t;
				hiX = c.x;
				hiDepth = depth;
			}
		}
	}
	return (hiX * loDepth + loX * hiDepth) / (loDepth + hiDepth);
}

/** CSS px from the top at which the reference shows, at a zoom and shift of a base field of view. */
function refAt(
	view: View,
	input: ComposeInput,
	x: number,
	fov: number,
	zoom: number,
	shiftPx: number,
) {
	const tan = Math.tan(THREE.MathUtils.degToRad(fov) / 2) / zoom;
	const px = input.height / (2 * tan);
	const ty = view.tan(arcReference(input.kind, x, tmp), t2).y;
	return input.height / 2 - ty * px - shiftPx;
}

const KEY_POS: number[] = [];
const KEY_AT: number[] = [];
/** Points `view` along the key camera at progress p, panned by `pan`. */
function aimAt(view: View, kind: LayoutKind, p: number, pan: number) {
	sampleKeysInto(cameraKeys[kind], p, "pos", KEY_POS);
	sampleKeysInto(cameraKeys[kind], p, "at", KEY_AT);
	tmpB.set(KEY_AT[0] + pan, KEY_AT[1], KEY_AT[2]);
	view.aim(tmp.set(KEY_POS[0] + pan, KEY_POS[1], KEY_POS[2]), tmpB);
}

/** The rest framing: the arc between the sub-head and the dock, from the camera at p = 0. */
function restFraming(input: ComposeInput, view: View): Framing {
	aimAt(view, input.kind, 0, 0);
	const fov = keyFov(input.kind, 0, input.width / input.height);
	const area = input.area;
	if (!area) {
		return {
			offset: [0, 0, 0],
			zoom: 1,
			shiftPx: 0,
			ref: refAt(view, input, 0, fov, 1, 0),
		};
	}
	const tanHalf = Math.tan(THREE.MathUtils.degToRad(fov) / 2);
	const extent = {
		top: view.tan(arcReference(input.kind, 0, tmp, 0), t2).y / tanHalf,
		base: view.tan(tmp.set(0, 0, 0.02), t2).y / tanHalf,
	};
	const fit = fitArc(
		extent,
		area.surface,
		input.height,
		input.kind === "portrait",
	);
	return {
		offset: [0, 0, 0],
		zoom: fit.zoom,
		shiftPx: fit.shiftPx,
		ref: refAt(view, input, 0, fov, fit.zoom, fit.shiftPx),
	};
}

export function compose(input: ComposeInput): Composition {
	const view = new View();
	const rest = restFraming(input, view);
	const { kind, height: h } = input;
	sampleKeysInto(cameraKeys[kind], 1, "pos", KEY_POS);
	const pan = arcCentre(input, tmpB.set(KEY_POS[0], KEY_POS[1], KEY_POS[2]));
	aimAt(view, kind, 1, pan);
	const fov = keyFov(kind, 1, input.width / h);
	const fit = solve(input, view, fov);
	const insets = input.area?.beneath ?? { top: 0, bottom: 0 };
	const free = input.area?.open ?? { top: 0, bottom: 0 };
	// With the band: the picture under the header, centred in any room left.
	const used = (fit.top - fit.bottom) * fit.scale;
	const room = h - insets.top - insets.bottom - 2 * MARGIN_Y;
	const bandTop = insets.top + MARGIN_Y + Math.max(0, (room - used) / 2);
	const bandShift = h / 2 - fit.top * fit.scale - bandTop;
	// Before it: the same picture, centred in the room the band will take a
	// part of, never so far down that the band fading in would meet it.
	const openRoom = h - free.top - free.bottom - 2 * MARGIN_Y;
	const openTop = free.top + MARGIN_Y + Math.max(0, (openRoom - used) / 2);
	const openShift = bandShift - clamp(openTop - bandTop, 0, settleMax(h));
	const offset = [pan, 0, 0] as const;
	return {
		rest,
		open: {
			offset,
			zoom: fit.zoom,
			shiftPx: openShift,
			ref: refAt(view, input, pan, fov, fit.zoom, openShift),
		},
		band: {
			offset,
			zoom: fit.zoom,
			shiftPx: bandShift,
			ref: refAt(view, input, pan, fov, fit.zoom, bandShift),
		},
		board: fit.board.placement,
		boardWidthPx: fit.board.width * fit.scale,
	};
}

/**
 * Composes for the stage as it is: recomputed when the size, the insets, the
 * layout or the board change, and read every frame by the camera and the board.
 */
export function createComposer(
	stage: Readonly<Stage>,
	windows: WindowsView,
	board: () => BoardFrame,
): Composer {
	let version = 0;
	const last = {
		version: -1,
		width: 0,
		height: 0,
		portrait: false,
		area: null as DescentSafeArea | null,
		span: 0,
		value: null as Composition | null,
	};
	return {
		get() {
			const frame = board();
			const fresh =
				last.value &&
				last.version === version &&
				last.width === stage.width &&
				last.height === stage.height &&
				last.portrait === stage.portrait &&
				last.area === stage.safeArea &&
				last.span === frame.span;
			if (fresh && last.value) return last.value;
			last.version = version;
			last.width = stage.width;
			last.height = stage.height;
			last.portrait = stage.portrait;
			last.area = stage.safeArea;
			last.span = frame.span;
			last.value = compose({
				width: Math.max(1, stage.width),
				height: Math.max(1, stage.height),
				area: stage.safeArea,
				kind: stage.portrait ? "portrait" : "landscape",
				windows,
				board: frame,
			});
			return last.value;
		},
		invalidate() {
			version++;
		},
	};
}
