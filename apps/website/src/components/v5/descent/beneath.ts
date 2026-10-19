import * as THREE from "three";
import { BOARD_METRICS, type Point, type Rect } from "../board/board-layout";
import { BOARD_PALETTE_DARK, type RGBA, edgeColor } from "../board/board-paint";
import {
	PULSE,
	type PulseFrame,
	createPulseFrame,
	pulseAt,
	pulseTint,
	pulseWidth,
} from "../board/board-pulse";
import {
	BOARD_STRING_KEYS,
	type BoardGrid,
	type BoardStrings,
	createBoard,
} from "../board/board-spec";
import { createBoardSurface, readBoardFonts } from "./beneath-board";
import { createDirector } from "./beneath-director";
import {
	type ColumnWindows,
	columnWindows,
	followBeat,
	followWindow,
} from "./beneath-follow";
import {
	type PlanHop,
	type ScenarioPlan,
	buildPlans,
	planLocal,
	storyBox,
} from "./beneath-plan";
import {
	type Srgb,
	type WorldPoint,
	createRibbonView,
	createRibbons,
} from "./beneath-ribbons";
import type {
	Beneath,
	BoardFrame,
	BoardPlacement,
	Composer,
	Frame,
	LayoutKind,
	ScenarioClock,
	SceneContext,
	WindowCues,
	Windows,
} from "./internal";
import { clamp, createResources, decay, lerp, pulse, smooth } from "./shared";

/** Share of a window's width between the docks of its two threads. */
const THREAD_SPREAD = 0.2;
const THREAD_SAMPLES = 33;
/**
 * Threads draw in one after another, the centre windows first: each starts
 * this share of the draw-in later than the one before it, and takes the rest.
 */
const THREAD_STAGGER = 0.4;
/** Opacity of the threads before the route lights, and while a changed route relights, against egui's normal reference wire. */
const THREAD_LEVEL = { base: 0.8, quiet: 0.5 } as const;
/** How strongly the route shows over the whisper board before it lights. */
const ROUTE_DIM = 0.45;
/** CSS px from the frame's sides within which a thread end fades out; past the side it is gone. */
const SIDE_FADE = 18;
/** How fast (per second) a phone's board eases toward the beat it follows. */
const FOLLOW_RATE = 2.2;
/** How fast (per second) a wide screen's board eases to frame a changed story. */
const STORY_RATE = 4;
/** Board units of air beside the columns a phone keeps in view. */
const FOLLOW_MARGIN = 12;
/** Share of a phone's width over which the board fades out at each side, and the comets with it. */
const EDGE_FADE = 0.1;
/**
 * The board below the story is not shown: it stays whole `keep` board units
 * under the story's lowest node, then fades out over `feather`, before the
 * next row.
 */
const STORY_CLIP = { keep: 8, feather: 40 } as const;
/**
 * On a phone the route is lit `clear` board units past its columns, then fades
 * out over `feather`; the lit window moves ahead of the sliding board (`rate`
 * per second), so nodes leaving the frame dim before its edge cuts them.
 */
const SPOT = { clear: 5, feather: 17, rate: 6 } as const;
/** Opacity of the selected app's own thread when the story does not touch that app: a hint, never lit. */
const OWN_THREAD = 0.14;
/** Texture pixels per board unit: phones see the board closer, at a higher density. */
const DENSITY: Record<LayoutKind, number> = { landscape: 2, portrait: 2.25 };
/** Rates (per second) at which a changed route dims out and the new one lights. */
const SWAP = { out: 7, in: 3.5 } as const;
/** The comet head: a hot core the bloom turns into a faint glow, plus a soft halo. */
const HEAD = { core: 0.5, coreHeat: 1.7, halo: 3.2, haloAlpha: 0.26 } as const;
/**
 * On the lit white execution wires the egui tint (wire pulled toward primary)
 * is darker than the wire, so those comets burn hotter instead: primary
 * lightened toward white, a core at least `corePx` CSS px in radius, and a
 * trail brighter than the wire under it.
 */
const HOT = {
	lighten: 0.7,
	corePx: 2.5,
	coreHeat: 1.8,
	trailHeat: 1.5,
} as const;
/**
 * The narrowest comet trail, in CSS px. egui sizes pulses for a board at 100%;
 * this board is seen far smaller, and its pulses are what the eye should follow.
 */
const COMET_MIN_PX = 2.4;
/** The pen that draws a thread in: a small soft light at its tip. */
const PEN = { radiusPx: 2.6, alpha: 0.5 } as const;

/** egui's reference wire: three strokes, [normal, selected] width (board units) and alpha. */
const REFERENCE_STROKES: readonly {
	width: readonly [number, number];
	alpha: readonly [number, number];
	lighten: number;
}[] = [
	{ width: [5, 8], alpha: [0.04, 0.08], lighten: 0 },
	{ width: [1.5, 2.5], alpha: [0.3, 0.5], lighten: 0 },
	{ width: [0.8, 1.2], alpha: [0.4, 0.7], lighten: 0.2 },
];

const srgb = (color: RGBA, lighten = 0): Srgb => [
	lerp(color[0], 255, lighten) / 255,
	lerp(color[1], 255, lighten) / 255,
	lerp(color[2], 255, lighten) / 255,
];

interface ThreadPath {
	readonly id: string;
	readonly app: number;
	offset: number;
	/** Top edge of the node's reference square, in board units. */
	anchor: Point;
	readonly samples: number[][];
	/** The same samples from the board up to the window. */
	readonly reversed: number[][];
	/** The drawn part during the draw-in; its last point is interpolated. */
	readonly drawn: number[][];
	/** At index k, `drawn`'s first k + 1 points, sharing its point arrays. */
	readonly prefixes: readonly (readonly number[][])[];
	/** Where in the draw-in order this thread starts, 0 (first) to 1 (last). */
	order: number;
	/** 0–1: both ends on screen. */
	weight: number;
	/** 0–1: how much of it is drawn. */
	length: number;
	/** 0–1: how far it shows as part of the scenario's story; other threads are not drawn. */
	show: number;
	/** 0–1: how far it is lit as part of the route. */
	lit: number;
}

function boardStrings(strings: Record<string, string>): BoardStrings {
	return Object.fromEntries(
		BOARD_STRING_KEYS.map((key) => [key, strings[key] ?? ""]),
	) as BoardStrings;
}

/** How far the board eases toward where it should lie this frame (1 when dt ≤ 0). */
const followEase = (dt: number, rate: number) =>
	dt <= 0 ? 1 : 1 - Math.exp(-dt * rate);

function cubic(p: readonly THREE.Vector3[], t: number, out: number[]) {
	const u = 1 - t;
	const a = u * u * u;
	const b = 3 * u * u * t;
	const c = 3 * u * t * t;
	const d = t * t * t;
	out[0] = a * p[0].x + b * p[1].x + c * p[2].x + d * p[3].x;
	out[1] = a * p[0].y + b * p[1].y + c * p[2].y + d * p[3].y;
	out[2] = a * p[0].z + b * p[1].z + c * p[2].z + d * p[3].z;
}

const ease = (t: number) => smooth(0, 1, t);

export function createBeneath(
	ctx: SceneContext,
	windows: Windows,
	composer: Composer,
): Beneath {
	const { camera, under, data, stage, renderer, options } = ctx;
	const res = createResources(ctx.maxAniso);
	const strings = boardStrings(data.strings);
	let grid: BoardGrid = "landscape";
	let spec = createBoard(strings, grid);
	const board = createBoardSurface(
		renderer,
		spec,
		readBoardFonts(ctx.container),
	);
	under.add(board.group);
	let plans: ScenarioPlan[] = buildPlans(spec, board.layout());
	/** Each scenario's route nodes as one box, in board units. */
	let stories: Rect[] = plans.map(storyBox);

	const view = createRibbonView();
	const threadBatch = createRibbons(res, view, 4096);
	const pulseBatch = createRibbons(res, view, 4096);
	threadBatch.mesh.renderOrder = 2;
	pulseBatch.mesh.renderOrder = 3;
	under.add(threadBatch.mesh, pulseBatch.mesh);

	const palette = BOARD_PALETTE_DARK;
	const threadColors = [
		srgb(palette.reference),
		srgb(palette.reference, 0.2),
	] as const;
	const threadTint = srgb(pulseTint(palette.reference, palette.primary));
	const penTint = srgb(palette.reference, 0.55);
	const primaryTint = srgb(palette.primary);
	const hotTint = srgb(palette.primary, HOT.lighten);
	const hotTrail: Srgb = [
		hotTint[0] * HOT.trailHeat,
		hotTint[1] * HOT.trailHeat,
		hotTint[2] * HOT.trailHeat,
	];
	let execEdges = new Set<string>();
	let edgeTints = new Map<string, Srgb>();
	function readEdges() {
		const edges = board.layout().edges;
		execEdges = new Set(
			edges.filter((edge) => edge.kind === "execution").map((edge) => edge.id),
		);
		edgeTints = new Map(
			edges.map((edge) => [
				edge.id,
				srgb(pulseTint(edgeColor(edge, palette), palette.primary)),
			]),
		);
	}
	readEdges();

	const threads: ThreadPath[] = spec.threads.map((thread) => {
		const samples = Array.from({ length: THREAD_SAMPLES }, () => [0, 0, 0]);
		const drawn = Array.from({ length: THREAD_SAMPLES }, () => [0, 0, 0]);
		return {
			id: thread.id,
			app: thread.app,
			offset: 0,
			anchor: [0, 0],
			samples,
			reversed: [...samples].reverse(),
			drawn,
			prefixes: drawn.map((_, k) => drawn.slice(0, k + 1)),
			order: 0,
			weight: 1,
			length: 0,
			show: 0,
			lit: 0,
		};
	});
	const threadById = new Map(threads.map((t) => [t.id, t]));
	const edgePaths = new Map<string, number[][]>();

	const arrivals = new Set<(app: number) => void>();
	const director = createDirector({
		emit: (event) => options.onScenarioEvent?.(event),
		reset: (apps) => {
			for (const app of apps) windows.setWindowState(app, "idle");
		},
		depart: (app) => windows.setWindowState(app, "live"),
		arrive: (app) => {
			for (const cb of arrivals) cb(app);
		},
	});

	let kind: LayoutKind = "landscape";
	/**
	 * Where the board lies, eased toward its story: the board x under the
	 * windows, the story's lowest edge and the room it leaves, and the centre of
	 * a phone's lit window.
	 */
	const aim = {
		x: Number.NaN,
		bottom: Number.NaN,
		slack: Number.NaN,
		spot: Number.NaN,
	};
	let threadYaw = Number.NaN;
	let geometryVersion = 0;
	let threadsWritten = -1;
	/** Route crossfade while a changed route dims out and the new one lights. */
	let swap = 1;
	/** How far the route has lit, which quiets the threads that are not lit with it. */
	let threadQuiet = 0;
	/** What the threads were last drawn from: geometry, plan, and each thread's quantized weight, length, story and light. */
	const drawnKey = { version: -1, plan: "" as string | null };
	const LEVELS = 4;
	const drawnLevels = new Int32Array(spec.threads.length * LEVELS).fill(-1);
	const tmp = new THREE.Vector3();
	const tmpB = new THREE.Vector3();
	const point: [number, number] = [0, 0];
	const at = (x: number, y: number): Point => {
		point[0] = x;
		point[1] = y;
		return point;
	};
	const curve = [
		new THREE.Vector3(),
		new THREE.Vector3(),
		new THREE.Vector3(),
		new THREE.Vector3(),
	];
	const flash = new Map<number, number>();
	/** One comet frame, rewritten for every running hop. */
	const cometFrame = createPulseFrame();
	const NONE: readonly number[] = [];
	const cueOut = { apps: NONE, arrivals: NONE, flash };

	function anchorsAndOffsets() {
		const layout = board.layout();
		const half = BOARD_METRICS.reference / 2;
		for (const t of threads) {
			const [x, y] = layout.threadAnchor(t.id);
			t.anchor = [x, y - half];
		}
		const byApp = new Map<number, ThreadPath[]>();
		for (const t of threads) byApp.set(t.app, [...(byApp.get(t.app) ?? []), t]);
		for (const [app, list] of byApp) {
			list.sort((a, b) => a.anchor[0] - b.anchor[0]);
			const width = windows.width(app);
			for (const [i, t] of list.entries()) {
				t.offset = (i - (list.length - 1) / 2) * THREAD_SPREAD * width;
			}
		}
		// The centre windows draw their threads first, the arc's ends last.
		const reach = threads.map((t) => Math.abs(windows.theta(t.app)));
		const far = Math.max(1e-6, ...reach);
		for (const [i, t] of threads.entries()) t.order = reach[i] / far;
	}

	// The wires in world space, for the comets; arrays are reused as the board moves.
	function writeEdgePaths() {
		for (const edge of board.layout().edges) {
			let path = edgePaths.get(edge.id);
			if (path?.length !== edge.points.length) {
				path = edge.points.map(() => [0, 0, 0]);
				edgePaths.set(edge.id, path);
			}
			for (const [i, p] of edge.points.entries()) {
				board.world(p, tmp).toArray(path[i]);
			}
		}
	}

	function place(next: BoardPlacement) {
		const now = board.placement();
		const moved =
			now.unit !== next.unit ||
			Math.abs(now.x - next.x) >= 1e-6 ||
			now.y !== next.y ||
			now.z !== next.z ||
			now.tilt !== next.tilt;
		if (!moved) return;
		board.place(next);
		writeEdgePaths();
		geometryVersion++;
	}

	// Vertical tangents at both ends: straight down from the dock, straight
	// down onto the top edge of the reference square.
	function writeThreads() {
		for (const t of threads) {
			const dock = windows.dock(t.app, t.offset, curve[0]);
			const end = board.world(t.anchor, curve[3]);
			const drop = clamp((dock.y - end.y) * 0.45, 0.2, 2.4);
			curve[1].set(dock.x, dock.y - drop, dock.z);
			curve[2].set(end.x, end.y + drop * 0.9, end.z);
			for (let i = 0; i < THREAD_SAMPLES; i++) {
				cubic(curve, i / (THREAD_SAMPLES - 1), t.samples[i]);
			}
		}
	}

	const nodeWidth = () =>
		board.layout().nodes[0]?.width ?? BOARD_METRICS.minWidth;
	const frameMemo = {
		layout: board.layout(),
		width: Number.NaN,
		frames: null as ColumnWindows | null,
	};

	/** The columns a phone keeps in view; null where the whole board shows. */
	function narrowFrames() {
		if (kind !== "portrait") return null;
		const boardWidth = board.layout().bounds.width;
		const width = Math.min(
			boardWidth,
			2 * nodeWidth() + BOARD_METRICS.gutter + 2 * FOLLOW_MARGIN,
		);
		if (frameMemo.layout !== board.layout() || frameMemo.width !== width) {
			frameMemo.layout = board.layout();
			frameMemo.width = width;
			frameMemo.frames = columnWindows(board.layout(), width);
		}
		return frameMemo.frames;
	}

	const boardFrame = {
		content: { x: 0, y: 0, width: 1, height: 1 },
		centre: 0,
		span: 1,
	};
	/** The board as the composition frames it: all of it, or a phone's columns. */
	function frame(): BoardFrame {
		const content = board.content();
		boardFrame.content = content;
		boardFrame.centre = content.x + content.width / 2;
		const frames = narrowFrames();
		boardFrame.span = frames ? frames.span + 2 * FOLLOW_MARGIN : content.width;
		return boardFrame;
	}

	/**
	 * Board x a phone centres under the selected window: the columns the beats
	 * land in once the layer runs, the first beat's before.
	 */
	function followTarget(
		plan: ScenarioPlan,
		clock: ScenarioClock,
		alive: boolean,
	) {
		const frames = narrowFrames();
		if (!frames) return frame().centre;
		const local = planLocal(plan, clock.elapsed);
		const moving = stage.motion > 0;
		const beat = alive || !moving ? followBeat(plan, local, moving) : 0;
		return followWindow(frames, plan, beat);
	}

	const target = { x: 0, bottom: 0, slack: 0 };
	/**
	 * Where the story should lie: centred under the windows, its lowest edge
	 * where the board's foot was composed, and the board height it leaves free.
	 */
	function storyTarget(index: number, clock: ScenarioClock, alive: boolean) {
		const content = board.content();
		const foot = content.y + content.height;
		const plan = plans[index];
		const story = stories[index];
		target.x = frame().centre;
		target.bottom = foot;
		target.slack = 0;
		if (!plan || !story) return target;
		const pad = BOARD_METRICS.reference / 2;
		target.bottom = story.y + story.height + pad;
		target.slack = Math.max(0, content.height - story.height - 2 * pad);
		target.x =
			kind === "portrait"
				? followTarget(plan, clock, alive)
				: story.x + story.width / 2;
		return target;
	}

	// The board lies where the composition puts it, moved in its own plane so
	// the story is what is framed: centred under the windows (on a phone, the
	// columns the beats land in) and, with the copy band, its lowest edge at
	// the board's composed foot, right above the band. Before the band a short
	// story sits in the middle of that room instead, and settles to the foot as
	// the windows rise for the band, so it stays put on screen. A changed story
	// eases there while its route is dark; what lies below the story is not
	// shown, and a phone lights only its columns.
	function follow(clock: ScenarioClock, f: Frame) {
		const placement = composer.get().board;
		const next = storyTarget(clock.index, clock, f.alive);
		const k = followEase(f.dt, kind === "portrait" ? FOLLOW_RATE : STORY_RATE);
		const toward = (from: number, to: number, rate = k) =>
			Number.isNaN(from) ? to : lerp(from, to, rate);
		aim.x = toward(aim.x, next.x);
		aim.bottom = toward(aim.bottom, next.bottom);
		aim.slack = toward(aim.slack, next.slack);
		aim.spot = toward(aim.spot, next.x, followEase(f.dt, SPOT.rate));
		const content = board.content();
		const atFoot = aim.bottom + ((1 - f.reveal.band) * aim.slack) / 2;
		const slide = (frame().centre - aim.x) * placement.unit;
		const drop = (content.y + content.height - atFoot) * placement.unit;
		place({
			...placement,
			x: placement.x + slide,
			y: placement.y - drop * Math.cos(placement.tilt),
			z: placement.z + drop * Math.sin(placement.tilt),
		});
		board.clip(aim.bottom + STORY_CLIP.keep, STORY_CLIP.feather);
		const frames = narrowFrames();
		if (!frames) {
			board.spotlight(null);
			return;
		}
		const half = frames.span / 2 + SPOT.clear;
		board.spotlight({
			left: aim.spot - half,
			right: aim.spot + half,
			feather: SPOT.feather,
		});
	}

	function updateView() {
		view.viewport.value.set(
			Math.max(1, stage.width),
			Math.max(1, stage.height),
		);
		view.dpr.value = Math.max(1, stage.dpr || 1);
		const drawn = stage.width * (stage.dpr || 1);
		board.edgeFade(kind === "portrait" ? drawn * EDGE_FADE : 0, drawn);
		const area = board.area();
		const midX = area.x + area.width / 2;
		const midY = area.y + area.height / 2;
		const a = board.world(at(midX, midY), tmp).project(camera);
		const b = board.world(at(midX + 1, midY), tmpB).project(camera);
		const zoom = Math.hypot(
			((b.x - a.x) * stage.width) / 2,
			((b.y - a.y) * stage.height) / 2,
		);
		view.zoom.value = zoom > 1e-4 ? zoom : 1;
	}

	/** 1 well inside the frame, fading over SIDE_FADE px to 0 at its sides and beyond. */
	function onScreen(p: THREE.Vector3) {
		const v = tmpB.copy(p).project(camera);
		const x = ((v.x + 1) / 2) * stage.width;
		return (
			smooth(0, SIDE_FADE, x) * smooth(stage.width, stage.width - SIDE_FADE, x)
		);
	}

	// A thread shows only while both its ends are in the frame: none enters
	// from outside it.
	function threadWeights() {
		for (const t of threads) {
			const [dx, dy, dz] = t.samples[0];
			const [ax, ay, az] = t.samples[THREAD_SAMPLES - 1];
			t.weight = onScreen(tmp.set(dx, dy, dz)) * onScreen(tmp.set(ax, ay, az));
		}
	}

	/** Eases a thread's story level toward `target`: in with the new route, out as fast as the old one dims. */
	const storyStep = (from: number, target: number, dt: number) => {
		if (dt <= 0) return target;
		if (target > from) return Math.min(target, from + dt * SWAP.in);
		return Math.max(target, from - dt * SWAP.out);
	};

	/**
	 * On a wide screen, the selected app's own thread when the story leaves that
	 * app out, so choosing it still shows where it meets the layer.
	 */
	function ownThread(plan: ScenarioPlan | null) {
		if (kind === "portrait" || !plan) return null;
		for (const id of plan.threads) {
			if (threadById.get(id)?.app === stage.app) return null;
		}
		return threads.find((t) => t.app === stage.app) ?? null;
	}

	// Only the scenario's own threads, from the app it starts in and to the
	// apps it delivers to, are drawn, and only once its route is painted, so a
	// changed story's threads never meet the old one's; they light with the route.
	function threadProgress(frame: Frame, plan: ScenarioPlan | null) {
		const drawIn = frame.reveal.threads;
		const painted = plan !== null && board.painted() === plan.id;
		const own = ownThread(plan);
		for (const t of threads) {
			const start = t.order * THREAD_STAGGER;
			t.length = ease((drawIn - start) / (1 - THREAD_STAGGER));
			const story = painted && (plan?.threads.has(t.id) ?? false);
			const shown = story ? 1 : t === own ? OWN_THREAD : 0;
			t.show = storyStep(t.show, shown, frame.dt);
			t.lit = story ? frame.reveal.lit * swap : 0;
		}
	}

	/** The drawn part of a thread: whole samples up to `length`, then one interpolated point. */
	function drawnPart(t: ThreadPath) {
		const last = THREAD_SAMPLES - 1;
		const reach = t.length * last;
		const whole = Math.floor(reach);
		for (let i = 0; i <= whole; i++) {
			const s = t.samples[i];
			const d = t.drawn[i];
			d[0] = s[0];
			d[1] = s[1];
			d[2] = s[2];
		}
		if (whole >= last) return t.samples;
		const f = reach - whole;
		const a = t.samples[whole];
		const b = t.samples[whole + 1];
		const d = t.drawn[whole + 1];
		d[0] = lerp(a[0], b[0], f);
		d[1] = lerp(a[1], b[1], f);
		d[2] = lerp(a[2], b[2], f);
		return t.prefixes[whole + 1];
	}

	/** Records level `k` of the threads; returns whether it changed. */
	function record(k: number, v: number) {
		const changed = drawnLevels[k] !== v;
		drawnLevels[k] = v;
		return changed;
	}

	/** Whether any thread's weight, length, story or light changed by a step that shows; records them. */
	function levelsChanged() {
		let changed = false;
		for (let i = 0; i < threads.length; i++) {
			const t = threads[i];
			const k = i * LEVELS;
			changed = record(k, Math.round(t.weight * 50)) || changed;
			changed = record(k + 1, Math.round(t.length * 200)) || changed;
			changed = record(k + 2, Math.round(t.show * 50)) || changed;
			changed = record(k + 3, Math.round(t.lit * 50)) || changed;
		}
		return changed;
	}

	/** How much of a thread shows: on screen and part of the story. */
	const presence = (t: ThreadPath) => t.weight * t.show;

	// A story thread lights up with the route; while it is not lit it stays at
	// egui's normal reference wire, quieter once the route has lit.
	function drawThread(t: ThreadPath) {
		const shown = presence(t);
		if (shown < 0.01 || t.length <= 0.002) return;
		const path = t.length >= 1 ? t.samples : drawnPart(t);
		const level = lerp(THREAD_LEVEL.base, THREAD_LEVEL.quiet, threadQuiet);
		for (const stroke of REFERENCE_STROKES) {
			const alpha =
				lerp(stroke.alpha[0] * level, stroke.alpha[1], t.lit) * shown;
			const width = lerp(stroke.width[0], stroke.width[1], t.lit);
			threadBatch.stroke(
				path,
				threadColors[stroke.lighten ? 1 : 0],
				alpha,
				width,
			);
		}
	}

	function drawThreads(plan: ScenarioPlan | null) {
		const id = plan?.id ?? null;
		const moved = levelsChanged();
		const unchanged =
			!moved && drawnKey.version === geometryVersion && drawnKey.plan === id;
		if (unchanged) return;
		drawnKey.version = geometryVersion;
		drawnKey.plan = id;
		threadBatch.clear();
		for (const t of threads) drawThread(t);
		threadBatch.commit();
	}

	// A pulse rides only a thread that has drawn in whole, so no comet floats
	// ahead of its thread while the layer comes alive.
	function hopPath(hop: PlanHop): readonly WorldPoint[] | null {
		if (hop.kind === "edge") return edgePaths.get(hop.id) ?? null;
		const t = threadById.get(hop.id);
		if (!t || t.weight < 0.5 || t.length < 0.999) return null;
		return hop.toApp ? t.reversed : t.samples;
	}

	const hopTint = (hop: PlanHop): Srgb =>
		hop.kind === "edge" ? (edgeTints.get(hop.id) ?? threadTint) : threadTint;
	const isHot = (hop: PlanHop) => hop.kind === "edge" && execEdges.has(hop.id);
	/** How much a hop's wire or thread shows, which its comet never outshines. */
	function hopPresence(hop: PlanHop) {
		if (hop.kind === "edge") return swap;
		const t = threadById.get(hop.id);
		return t ? presence(t) : 0;
	}
	/** 1 inside a phone's frame, fading out at its sides like the board; 1 on a wide screen. */
	function sideFade(p: WorldPoint) {
		if (kind !== "portrait") return 1;
		const v = tmpB.set(p[0], p[1], p[2]).project(camera);
		const x = ((v.x + 1) / 2) * stage.width;
		const edge = stage.width * EDGE_FADE;
		return smooth(0, edge, x) * smooth(stage.width, stage.width - edge, x);
	}

	function head(p: WorldPoint, width: number, tint: Srgb, o: number) {
		pulseBatch.disc(p, width * HEAD.halo, tint, HEAD.haloAlpha * o, "soft");
		pulseBatch.disc(p, width * PULSE.headRadius, tint, PULSE.headAlpha * o);
		pulseBatch.disc(p, width * HEAD.core, tint, o, "hard", HEAD.coreHeat);
	}

	function hotHead(p: WorldPoint, width: number, o: number) {
		const core = Math.max(
			width * PULSE.headRadius,
			HOT.corePx / view.zoom.value,
		);
		pulseBatch.disc(
			p,
			width * HEAD.halo,
			primaryTint,
			HEAD.haloAlpha * o,
			"soft",
		);
		pulseBatch.disc(p, core, hotTint, o, "hard", HOT.coreHeat);
	}

	// egui's activation pulse: a short trail behind the head, with a soft glow.
	function comet(f: PulseFrame, tint: Srgb, hot: boolean, shown: number) {
		const zoom = view.zoom.value;
		const px = Math.max(pulseWidth(zoom), COMET_MIN_PX);
		const o = f.opacity * shown * sideFade(f.head);
		pulseBatch.stroke(
			f.trail,
			hot ? primaryTint : tint,
			PULSE.glowAlpha * o,
			(px + PULSE.glowExtraWidth) / zoom,
		);
		pulseBatch.stroke(
			f.trail,
			hot ? hotTrail : tint,
			PULSE.trailAlpha * o,
			px / zoom,
		);
		if (hot) hotHead(f.head, px / zoom, o);
		else head(f.head, px / zoom, tint, o);
	}

	// Reduced motion: a long exposure. Every hop is the whole streak its comet
	// leaves, brightest mid-run, with the head resting where the work lands.
	function exposure(path: readonly WorldPoint[], tint: Srgb, hot: boolean) {
		const zoom = view.zoom.value;
		const px = Math.max(pulseWidth(zoom), COMET_MIN_PX);
		const last = Math.max(1, path.length - 1);
		const glow = path.map(
			(p, i) =>
				(0.3 + 0.7 * Math.sin((Math.PI * i) / last)) *
				PULSE.glowAlpha *
				sideFade(p),
		);
		const trail = glow.map((v) => (v / PULSE.glowAlpha) * PULSE.trailAlpha);
		const glowWidth = (px + PULSE.glowExtraWidth) / zoom;
		pulseBatch.stroke(path, hot ? primaryTint : tint, glow, glowWidth);
		pulseBatch.stroke(path, hot ? hotTrail : tint, trail, px / zoom);
		const end = sideFade(path[last]);
		if (hot) hotHead(path[last], px / zoom, end);
		else head(path[last], px / zoom, tint, end);
	}

	// While a thread draws in, a small light rides its tip.
	function pens() {
		const radius = PEN.radiusPx / view.zoom.value;
		for (const t of threads) {
			const shown = presence(t);
			if (t.length <= 0.002 || t.length >= 0.999 || shown < 0.01) continue;
			const tip =
				t.drawn[
					Math.min(
						THREAD_SAMPLES - 1,
						Math.ceil(t.length * (THREAD_SAMPLES - 1)),
					)
				];
			const o =
				PEN.alpha *
				shown *
				smooth(0.02, 0.14, t.length) *
				(1 - smooth(0.85, 0.99, t.length));
			pulseBatch.disc(tip, radius, penTint, o, "soft");
		}
	}

	// The running scenario's pulses; the still draws each hop as its long exposure.
	function hops(plan: ScenarioPlan, clock: ScenarioClock) {
		const local = planLocal(plan, clock.elapsed);
		for (const hop of plan.hops) {
			const path = hopPath(hop);
			const shown = hopPresence(hop);
			if (!path || shown < 0.01) continue;
			if (!stage.motion) {
				exposure(path, hopTint(hop), isHot(hop));
				continue;
			}
			const f = pulseAt(path, (local - hop.start) / hop.duration, cometFrame);
			if (f) comet(f, hopTint(hop), isHot(hop), shown);
		}
	}

	function drawPulses(active: ScenarioPlan | null, clock: ScenarioClock) {
		pulseBatch.clear();
		if (stage.motion) pens();
		if (active) hops(active, clock);
		pulseBatch.commit();
	}

	// The route dims back into the whisper board; once it is out (or the board
	// does not show), the new one is painted.
	function swapRoute(wanted: ScenarioPlan["id"] | null, frame: Frame) {
		swap = frame.dt <= 0 ? 0 : decay(swap, 0, frame.dt, SWAP.out);
		if (swap > 0.02 && frame.reveal.board > 0.002) return;
		board.paint(wanted);
		if (frame.dt <= 0) swap = 1;
	}

	// The scenario's route rises dim with the board and lights with the layer;
	// a changed route dims out, is painted, then shows again: the board never jumps.
	function paintRoute(plan: ScenarioPlan | null, frame: Frame) {
		const wanted = plan?.id ?? null;
		if (board.painted() !== wanted) swapRoute(wanted, frame);
		else swap = frame.dt <= 0 ? 1 : Math.min(1, swap + frame.dt * SWAP.in);
		board.show(frame.reveal.board, lerp(ROUTE_DIM, 1, frame.reveal.lit) * swap);
		threadQuiet = frame.reveal.lit;
	}

	function direct(active: ScenarioPlan | null, clock: ScenarioClock) {
		if (active && stage.motion) director.play(active, clock);
		else director.idle();
	}

	function rewriteThreads() {
		if (threadsWritten === geometryVersion) return;
		threadsWritten = geometryVersion;
		writeThreads();
	}

	/** Everything read from the board's layout, after it was laid out again. */
	function relaid() {
		plans = buildPlans(spec, board.layout());
		stories = plans.map(storyBox);
		readEdges();
		edgePaths.clear();
		composer.invalidate();
	}

	// A phone lays the board out on its own grid, where each story fits its frame best.
	function applyLayout(portrait: boolean) {
		kind = portrait ? "portrait" : "landscape";
		board.density(DENSITY[kind]);
		if (grid !== kind) {
			grid = kind;
			spec = createBoard(strings, grid);
			board.regrid(spec, grid);
			relaid();
		}
		anchorsAndOffsets();
		aim.x = Number.NaN;
		aim.bottom = Number.NaN;
		aim.slack = Number.NaN;
		aim.spot = Number.NaN;
		threadYaw = Number.NaN;
		geometryVersion++;
		writeEdgePaths();
	}

	const raise = (app: number, v: number) =>
		flash.set(app, Math.max(flash.get(app) ?? 0, v));

	// Edge flashes as work leaves an app and as it lands in one.
	function flashWindows(plan: ScenarioPlan, local: number) {
		if (!stage.motion) return;
		for (const { app, at: when } of plan.departures)
			raise(app, pulse(local - when, 0.6) * 0.8);
		for (const { app, at: when } of plan.arrivals)
			raise(app, pulse(local - when, 1.1));
	}

	function cues(clock: ScenarioClock): WindowCues {
		const plan = plans[clock.index];
		flash.clear();
		cueOut.apps = plan?.apps ?? NONE;
		cueOut.arrivals = plan?.delivered ?? NONE;
		if (plan) flashWindows(plan, planLocal(plan, clock.elapsed));
		return cueOut;
	}

	return {
		layout: applyLayout,
		followWindows() {
			if (Math.abs(stage.arcYaw - threadYaw) > 1e-4) {
				threadYaw = stage.arcYaw;
				geometryVersion++;
			}
		},
		cues,
		// The board and the threads show the scenario's story as they reveal;
		// nothing runs until the layer comes alive.
		update(f, clock) {
			const plan = plans[clock.index] ?? null;
			const active = f.alive ? plan : null;
			under.visible = f.reveal.board > 0.002 || f.reveal.threads > 0;
			direct(active, clock);
			paintRoute(plan, f);
			follow(clock, f);
			updateView();
			rewriteThreads();
			threadWeights();
			threadProgress(f, plan);
			drawThreads(plan);
			drawPulses(active, clock);
		},
		redraw() {
			board.relayout();
			relaid();
			applyLayout(kind === "portrait");
		},
		frame,
		onArrive(cb) {
			arrivals.add(cb);
			return () => {
				arrivals.delete(cb);
			};
		},
		dispose() {
			res.dispose();
			board.dispose();
		},
	};
}
