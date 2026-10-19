// The app window faces, drawn into canvas textures.
import { type MeasureAt, fitLine, fitLines } from "./canvas-text";
import type { WindowState } from "./internal";
import type { DescentApp, DescentArrivals } from "./types";

export const T = {
	bg: "#fcf9f8",
	fg: "#121216",
	card: "#ffffff",
	muted: "#efeae7",
	mutedFg: "#666670",
	border: "#e2dad5",
	primary: "#fb562d",
	accent: "#f6472c",
	body: "#f5f1ef",
	dBg: "#0c0e13",
	dFg: "#f3efec",
	dCard: "#13161b",
	dMuted: "#22252b",
	dMutedFg: "#999ea8",
	dBorder: "#282c33",
	green: "#2f9a68",
	amber: "#d2952a",
} as const;

let SANS = '"Inter", ui-sans-serif, system-ui, sans-serif';
let MONO = "ui-monospace, SFMono-Regular, Menlo, monospace";
let LOCALE: string | undefined;

// Canvas text must use the families the host page renders with, so the scene
// reads them from the container's computed style instead of assuming names.
export function configureFonts(fonts: {
	sans?: string;
	mono?: string;
	locale?: string;
}) {
	if (fonts.sans) SANS = fonts.sans;
	if (fonts.mono) MONO = fonts.mono;
	if (fonts.locale) LOCALE = fonts.locale;
}

export function fontSpecs() {
	return [
		`400 14px ${SANS}`,
		`500 14px ${SANS}`,
		`600 14px ${SANS}`,
		`700 14px ${SANS}`,
		`500 12px ${MONO}`,
		`600 12px ${MONO}`,
	];
}

// Design pixels per world unit. At the resting camera one design pixel is
// roughly one CSS pixel on a 1440px wide viewport.
export const UNIT = 110;
export const TEX_SCALE = 2.5;

export type AppKind =
	| "ops"
	| "finance"
	| "customer"
	| "graph"
	| "control"
	| "inventory"
	| "agent"
	| "field";

export interface AppSpec {
	readonly kind: AppKind;
	readonly w: number;
	readonly h: number;
	readonly dark?: boolean;
	readonly phone?: boolean;
}

export const APP_SPECS: readonly AppSpec[] = [
	{ kind: "ops", w: 380, h: 236 },
	{ kind: "finance", w: 300, h: 216 },
	{ kind: "customer", w: 332, h: 222 },
	{ kind: "graph", w: 262, h: 222 },
	{ kind: "control", w: 300, h: 214 },
	{ kind: "inventory", w: 292, h: 210 },
	{ kind: "agent", w: 262, h: 236, dark: true },
	{ kind: "field", w: 132, h: 258, phone: true },
];

/** App indices from left to right along the arc. */
export const ARC_ORDER: readonly number[] = [5, 3, 6, 0, 2, 4, 1, 7];

/** Mock data inside the app faces, shared by the canvas drawers and the static fallback. */
export const FIXTURES = {
	opsTiles: ["98.4%", "214", "6"],
	opsTilesAfter: ["98.9%", "231", "4"],
	opsDeltas: ["+0.5", "+17", "−2"],
	opsBars: [43, 58, 49, 73, 62, 88, 78, 95],
	opsBarsAfter: [52, 66, 61, 84, 75, 94, 90, 100],
	financeRows: [
		["PO–4482", "€24,800"],
		["PO–4481", "€8,240"],
		["PO–4479", "€61,020"],
	],
	customer: { initials: "AK", account: "Acme Industries", score: "92" },
	inventoryRows: [
		["Berlin", 72, "12.4k"],
		["Porto", 48, "8.1k"],
		["Warsaw", 86, "17.8k"],
		["Lyon", 61, "9.6k"],
	],
	field: { stop: "3", site: "Turbine B–14", eta: "14:30 · 4.2 km" },
} as const;

type Ctx = CanvasRenderingContext2D;
type Radii = number | number[];
type Rect = readonly [x0: number, y0: number, x1: number, y1: number];

interface TextOpts {
	size?: number;
	weight?: number;
	mono?: boolean;
	color?: string;
	align?: CanvasTextAlign;
	/** Width to fit: the text shrinks down to `min`, then ellipsizes. */
	max?: number;
	/** Smallest size a fitted string may shrink to. */
	min?: number;
	track?: number;
}

/** What a window face shows besides its app: its state and the end-state strings. */
export interface AppFace {
	readonly state: WindowState;
	readonly arrivals: DescentArrivals;
}

/** A 2D canvas for a face of w × h design px; `target` is cleared and reused when given. */
export function makeCanvas(
	w: number,
	h: number,
	scale: number,
	target?: HTMLCanvasElement,
) {
	const c = target ?? document.createElement("canvas");
	c.width = Math.round(w * scale);
	c.height = Math.round(h * scale);
	const ctx = c.getContext("2d");
	if (!ctx) {
		throw new Error(
			`Descent: no 2D canvas context for a ${c.width}×${c.height} texture`,
		);
	}
	ctx.scale(scale, scale);
	ctx.textBaseline = "alphabetic";
	ctx.imageSmoothingQuality = "high";
	return { c, ctx };
}

export function rr(
	ctx: Ctx,
	x: number,
	y: number,
	w: number,
	h: number,
	r: Radii,
) {
	ctx.beginPath();
	ctx.roundRect(x, y, w, h, r);
}

export function setFont(ctx: Ctx, size: number, weight = 500, mono = false) {
	ctx.font = `${weight} ${size}px ${mono ? MONO : SANS}`;
}

const floorFor = (size: number) => Math.max(9.5, size * 0.84);

function trackedWidth(ctx: Ctx, str: string, track: number) {
	let w = -track;
	for (const ch of str) w += ctx.measureText(ch).width + track;
	return w;
}

function measurer(
	ctx: Ctx,
	weight: number,
	mono = false,
	track = 0,
): MeasureAt {
	return (s, size) => {
		setFont(ctx, size, weight, mono);
		return track ? trackedWidth(ctx, s, track) : ctx.measureText(s).width;
	};
}

function trackedStart(x: number, w: number, align?: CanvasTextAlign) {
	if (align === "right") return x - w;
	if (align === "center") return x - w / 2;
	return x;
}

// Letter-spaced text, drawn glyph by glyph.
function trackedText(ctx: Ctx, str: string, x: number, y: number, o: TextOpts) {
	const track = o.track ?? 0;
	let cx = trackedStart(x, trackedWidth(ctx, str, track), o.align);
	ctx.textAlign = "left";
	for (const ch of str) {
		ctx.fillText(ch, cx, y);
		cx += ctx.measureText(ch).width + track;
	}
}

/** Draws one line; with `max` it is fitted (shrunk to `min`, then ellipsized). Returns its width. */
export function text(
	ctx: Ctx,
	s: string,
	x: number,
	y: number,
	o: TextOpts = {},
) {
	const { size = 13, weight = 500, color = T.fg, align = "left" } = o;
	let str = String(s);
	let px = size;
	if (o.max) {
		const fitted = fitLine(measurer(ctx, weight, o.mono, o.track), str, o.max, {
			size,
			floor: o.min ?? floorFor(size),
			locale: LOCALE,
		});
		str = fitted.text;
		px = fitted.size;
	}
	setFont(ctx, px, weight, o.mono);
	ctx.fillStyle = color;
	ctx.textAlign = align;
	if (o.track) trackedText(ctx, str, x, y, o);
	else ctx.fillText(str, x, y);
	return o.track ? trackedWidth(ctx, str, o.track) : ctx.measureText(str).width;
}

function wrap(
	ctx: Ctx,
	s: string,
	at: { x: number; y: number; maxW: number; lh: number; maxLines: number },
	o: TextOpts,
) {
	const size = o.size ?? 13;
	const weight = o.weight ?? 500;
	const fitted = fitLines(measurer(ctx, weight, o.mono), String(s), at.maxW, {
		size,
		floor: o.min ?? floorFor(size),
		maxLines: at.maxLines,
		locale: LOCALE,
	});
	const lh = (at.lh * fitted.size) / size;
	for (const [i, l] of fitted.lines.entries()) {
		text(ctx, l, at.x, at.y + i * lh, {
			...o,
			size: fitted.size,
			max: undefined,
		});
	}
}

function emberGradient(
	ctx: Ctx,
	x0: number,
	y0: number,
	x1: number,
	y1: number,
) {
	const g = ctx.createLinearGradient(x0, y0, x1, y1);
	g.addColorStop(0, "#ff7a45");
	g.addColorStop(0.55, T.primary);
	g.addColorStop(1, "#e2361d");
	return g;
}

type Shadow = readonly [blur: number, offsetY: number, alpha: number];

const TILE_SHADOW: Shadow = [10, 3, 0.05];
const ROW_SHADOW: Shadow = [8, 2, 0.045];

function softShadow(ctx: Ctx, blur: number, y: number, alpha = 0.08) {
	ctx.shadowColor = `rgba(18,18,22,${alpha})`;
	ctx.shadowBlur = blur;
	ctx.shadowOffsetY = y;
}

function noShadow(ctx: Ctx) {
	ctx.shadowColor = "transparent";
	ctx.shadowBlur = 0;
	ctx.shadowOffsetY = 0;
}

function check(
	ctx: Ctx,
	x: number,
	y: number,
	s: number,
	color: string,
	lw = 1.7,
) {
	ctx.save();
	ctx.strokeStyle = color;
	ctx.lineWidth = lw;
	ctx.lineCap = "round";
	ctx.lineJoin = "round";
	ctx.beginPath();
	ctx.moveTo(x, y + s * 0.52);
	ctx.lineTo(x + s * 0.36, y + s * 0.86);
	ctx.lineTo(x + s, y + s * 0.12);
	ctx.stroke();
	ctx.restore();
}

function appIcon(
	ctx: Ctx,
	x: number,
	y: number,
	s: number,
	letter: string,
	dark: boolean,
) {
	rr(ctx, x, y, s, s, s * 0.3);
	ctx.fillStyle = emberGradient(ctx, x, y, x + s, y + s);
	ctx.fill();
	ctx.fillStyle = "rgba(255,255,255,0.22)";
	rr(ctx, x, y, s, s * 0.5, [s * 0.3, s * 0.3, 0, 0]);
	ctx.fill();
	text(ctx, letter, x + s / 2, y + s * 0.69, {
		size: s * 0.56,
		weight: 750,
		color: "#fff",
		align: "center",
	});
	if (dark) {
		ctx.strokeStyle = "rgba(255,255,255,0.12)";
		rr(ctx, x + 0.5, y + 0.5, s - 1, s - 1, s * 0.3);
		ctx.stroke();
	}
}

interface Status {
	readonly label: string;
	readonly live: boolean;
}

const statusOf = (face: AppFace): Status => ({
	label: face.state === "idle" ? face.arrivals.ready : face.arrivals.live,
	live: face.state !== "idle",
});

const PILL_TEXT = { size: 10.5, weight: 600, mono: true, track: 0.9 } as const;

function pillWidth(ctx: Ctx, label: string, maxW: number) {
	const w = measurer(
		ctx,
		PILL_TEXT.weight,
		true,
		PILL_TEXT.track,
	)(label, PILL_TEXT.size);
	return Math.min(maxW, Math.ceil(w) + 30);
}

// READY / LIVE: a dot and a mono caption on a soft pill, ember once work arrived.
interface PillColors {
	readonly fill: string;
	readonly dot: string;
	readonly text: string;
	readonly halo?: string;
}

// READY is a quiet grey pill; LIVE is ember, with a soft halo around its dot.
const PILL_COLORS: Record<
	"light" | "dark",
	Record<"ready" | "live", PillColors>
> = {
	light: {
		ready: { fill: T.muted, dot: "#b9b0ab", text: T.mutedFg },
		live: {
			fill: "rgba(251,86,45,0.12)",
			dot: T.primary,
			text: "#cf3a1b",
			halo: "rgba(251,86,45,0.22)",
		},
	},
	dark: {
		ready: { fill: T.dMuted, dot: "#6b7079", text: T.dMutedFg },
		live: {
			fill: "rgba(251,86,45,0.2)",
			dot: T.primary,
			text: "#ffb59c",
			halo: "rgba(251,86,45,0.22)",
		},
	},
};

function dot(ctx: Ctx, x: number, y: number, r: number, fill: string) {
	ctx.beginPath();
	ctx.arc(x, y, r, 0, Math.PI * 2);
	ctx.fillStyle = fill;
	ctx.fill();
}

function statusPill(
	ctx: Ctx,
	status: Status,
	right: number,
	y: number,
	dark: boolean,
	maxW: number,
) {
	const colors =
		PILL_COLORS[dark ? "dark" : "light"][status.live ? "live" : "ready"];
	const w = pillWidth(ctx, status.label, maxW);
	const x = right - w;
	rr(ctx, x, y, w, 20, 10);
	ctx.fillStyle = colors.fill;
	ctx.fill();
	if (colors.halo) dot(ctx, x + 12, y + 10, 5.5, colors.halo);
	dot(ctx, x + 12, y + 10, 3, colors.dot);
	text(ctx, status.label, x + 21, y + 14, {
		...PILL_TEXT,
		color: colors.text,
		max: w - 27,
	});
	return w;
}

function header(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	dark: boolean,
	face: AppFace,
) {
	const fg = dark ? T.dFg : T.fg;
	appIcon(ctx, 11, 8, 20, app.name.charAt(0), dark);
	const status = statusOf(face);
	const pill = status.label
		? statusPill(ctx, status, w - 11, 8, dark, w * 0.42) + 10
		: 0;
	text(ctx, app.name, 40, 23, {
		size: 13.5,
		weight: 650,
		color: fg,
		max: w - 40 - pill - 11,
		min: 11.5,
	});
	ctx.fillStyle = dark ? T.dBorder : T.border;
	ctx.fillRect(0, 36, w, 1);
}

// A white card with a soft shadow and a hairline border.
function card(
	ctx: Ctx,
	x: number,
	y: number,
	w: number,
	h: number,
	shadow: Shadow,
	fill: string = T.card,
	stroke: string = T.border,
) {
	softShadow(ctx, ...shadow);
	rr(ctx, x, y, w, h, 9);
	ctx.fillStyle = fill;
	ctx.fill();
	noShadow(ctx);
	ctx.strokeStyle = stroke;
	ctx.lineWidth = 1;
	rr(ctx, x + 0.5, y + 0.5, w - 1, h - 1, 9);
	ctx.stroke();
}

function opsTiles(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	top: number,
	after: boolean,
) {
	const labels = [app.ui.onTime, app.ui.liveRuns, app.ui.attention];
	const values = after ? FIXTURES.opsTilesAfter : FIXTURES.opsTiles;
	const tw = (w - 24 - 16) / 3;
	for (const [i, v] of values.entries()) {
		const l = labels[i];
		const x = 12 + i * (tw + 8);
		card(ctx, x, top + 10, tw, 56, TILE_SHADOW);
		text(ctx, v, x + 11, top + 37, {
			size: 22,
			weight: 690,
			color: i === 2 ? T.primary : T.fg,
			track: -0.4,
		});
		// After an arrival the change sits on the label line, right-aligned.
		const delta = after
			? text(ctx, FIXTURES.opsDeltas[i], x + tw - 9, top + 55, {
					size: 11,
					weight: 650,
					color: T.green,
					align: "right",
				}) + 6
			: 0;
		text(ctx, l, x + 11, top + 55, {
			size: 11.5,
			weight: 520,
			color: T.mutedFg,
			max: tw - 20 - delta,
		});
	}
}

function opsChart(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	cy: number,
	ch: number,
	after: boolean,
) {
	rr(ctx, 12, cy, w - 24, ch, 10);
	ctx.fillStyle = "#15161b";
	ctx.fill();
	ctx.strokeStyle = "rgba(255,255,255,0.06)";
	for (let i = 1; i < 4; i++) {
		const gy = cy + 10 + ((ch - 20) * i) / 4;
		ctx.beginPath();
		ctx.moveTo(24, gy + 0.5);
		ctx.lineTo(w - 24, gy + 0.5);
		ctx.stroke();
	}
	text(ctx, app.ui.liveRuns.toUpperCase(), 24, cy + 20, {
		size: 10.5,
		weight: 600,
		mono: true,
		color: "rgba(243,239,236,0.55)",
		track: 1.1,
		max: w - 48,
	});
	const bw = (w - 48 - 7 * 8) / 8;
	const base = cy + ch - 10;
	const maxH = ch - 38;
	const bars = after ? FIXTURES.opsBarsAfter : FIXTURES.opsBars;
	for (const [i, b] of bars.entries()) {
		const bh = (maxH * b) / 100;
		const g = ctx.createLinearGradient(0, base - bh, 0, base);
		g.addColorStop(0, after ? "#ff9465" : "#ff7d4d");
		g.addColorStop(1, "#e23a1f");
		rr(ctx, 24 + i * (bw + 8), base - bh, bw, bh, [4, 4, 1, 1]);
		ctx.fillStyle = g;
		ctx.fill();
	}
}

function opsBody(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	h: number,
	face: AppFace,
) {
	const top = 37;
	const cy = top + 76;
	const after = face.state === "arrived";
	opsTiles(ctx, app, w, top, after);
	opsChart(ctx, app, w, cy, h - cy - 11, after);
}

// The finance row's action: a button, or a green chip once the request is approved.
function financeAction(
	ctx: Ctx,
	label: string,
	right: number,
	y: number,
	kind: FinanceAction,
) {
	const bw =
		kind === "approved"
			? Math.min(
					118,
					Math.max(72, Math.ceil(measurer(ctx, 640)(label, 11.5)) + 34),
				)
			: 72;
	const x = right - bw;
	rr(ctx, x, y + 11, bw, 26, 7);
	ctx.fillStyle =
		kind === "hot"
			? emberGradient(ctx, x, y, x + bw, y + 26)
			: kind === "approved"
				? T.green
				: T.fg;
	ctx.fill();
	if (kind === "approved") {
		check(ctx, x + 10, y + 19.5, 9, "#fff", 1.8);
		text(ctx, label, x + 24, y + 28.5, {
			size: 11.5,
			weight: 640,
			color: "#fff",
			max: bw - 31,
		});
	} else {
		text(ctx, label, x + bw / 2, y + 28.5, {
			size: 11.5,
			weight: 640,
			color: "#fff",
			align: "center",
			max: bw - 10,
		});
	}
	return bw;
}

type FinanceAction = "hot" | "plain" | "approved";

// Row 1 (PO–4481) is the request the approval scenario decides.
function financeRow(
	app: DescentApp,
	face: AppFace,
	i: number,
): { kind: FinanceAction; label: string } {
	if (i !== 1) return { kind: "plain", label: app.ui.review };
	if (face.state === "arrived") {
		return { kind: "approved", label: face.arrivals.approved };
	}
	return { kind: "hot", label: app.ui.approve };
}

function financeBody(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	_h: number,
	face: AppFace,
) {
	const x = 12;
	const rw = w - 24;
	for (const [i, [id, amt]] of FIXTURES.financeRows.entries()) {
		const y = 47 + i * 55;
		const row = financeRow(app, face, i);
		const done = row.kind === "approved";
		card(
			ctx,
			x,
			y,
			rw,
			48,
			ROW_SHADOW,
			done ? "#f3faf6" : T.card,
			done ? "#bfe0cb" : T.border,
		);
		text(ctx, id, x + 12, y + 21, { size: 13.5, weight: 680, color: T.fg });
		text(ctx, app.ui.dept, x + 12, y + 37, {
			size: 11.5,
			weight: 500,
			color: T.mutedFg,
			max: rw * 0.42,
		});
		const bw = financeAction(ctx, row.label, x + rw - 9, y, row.kind);
		text(ctx, amt, x + rw - bw - 19, y + 29, {
			size: 14,
			weight: 700,
			color: T.fg,
			align: "right",
		});
	}
}

function customerBody(ctx: Ctx, app: DescentApp, w: number, h: number) {
	ctx.fillStyle = "#16171c";
	ctx.fillRect(0, 37, 42, h - 37);
	for (let i = 0; i < 4; i++) {
		rr(ctx, 11, 50 + i * 30, 20, 20, 6);
		ctx.fillStyle = i === 1 ? "rgba(251,86,45,0.9)" : "rgba(255,255,255,0.13)";
		ctx.fill();
	}
	const x0 = 56;
	const { initials, account, score } = FIXTURES.customer;
	ctx.beginPath();
	ctx.arc(x0 + 18, 72, 18, 0, Math.PI * 2);
	ctx.fillStyle = "#fde9e2";
	ctx.fill();
	text(ctx, initials, x0 + 18, 76.5, {
		size: 12.5,
		weight: 720,
		color: "#c2361b",
		align: "center",
	});
	text(ctx, account, x0 + 46, 68, {
		size: 15,
		weight: 690,
		color: T.fg,
		max: w - x0 - 58,
	});
	text(ctx, `${app.ui.healthScore} · ${score}`, x0 + 46, 86, {
		size: 11.5,
		weight: 520,
		color: T.mutedFg,
		max: w - x0 - 58,
	});
	rr(ctx, x0, 106, w - x0 - 14, 7, 4);
	ctx.fillStyle = T.muted;
	ctx.fill();
	rr(ctx, x0, 106, (w - x0 - 14) * 0.92, 7, 4);
	ctx.fillStyle = emberGradient(ctx, x0, 0, w, 0);
	ctx.fill();
	rr(ctx, x0, 126, w - x0 - 14, h - 126 - 14, 9);
	ctx.fillStyle = "#fdeee8";
	ctx.fill();
	text(ctx, "✦", x0 + 12, 148, { size: 13, weight: 600, color: T.primary });
	wrap(
		ctx,
		app.ui.briefing,
		{ x: x0 + 30, y: 147, maxW: w - x0 - 58, lh: 17, maxLines: 3 },
		{ size: 12.5, weight: 560, color: "#3a2a26" },
	);
}

function graphChip(
	ctx: Ctx,
	label: string,
	x: number,
	y: number,
	hub: boolean,
	maxW: number,
) {
	const fitted = fitLine(measurer(ctx, 660), label, maxW - 22, {
		size: 12.5,
		floor: 10.5,
		locale: LOCALE,
	});
	setFont(ctx, fitted.size, 660);
	const tw = ctx.measureText(fitted.text).width + 22;
	softShadow(ctx, 10, 3, hub ? 0.18 : 0.08);
	rr(ctx, x - tw / 2, y - 15, tw, 30, 8);
	ctx.fillStyle = hub ? T.fg : T.card;
	ctx.fill();
	noShadow(ctx);
	if (!hub) {
		ctx.strokeStyle = T.border;
		rr(ctx, x - tw / 2 + 0.5, y - 14.5, tw - 1, 29, 8);
		ctx.stroke();
	}
	text(ctx, fitted.text, x, y + 4.5, {
		size: fitted.size,
		weight: 660,
		color: hub ? "#fff" : T.fg,
		align: "center",
	});
}

function graphBody(ctx: Ctx, app: DescentApp, w: number, h: number) {
	ctx.fillStyle = "#d9d1cc";
	for (let y = 48; y < h - 4; y += 15) {
		for (let x = 9; x < w - 4; x += 15) ctx.fillRect(x, y, 1.4, 1.4);
	}
	const hx = w * 0.5;
	const hy = h * 0.6;
	const spokes: [string, number, number][] = [
		[app.ui.contract, w * 0.24, h * 0.33],
		[app.ui.product, w * 0.77, h * 0.31],
		[app.ui.interaction, w * 0.72, h * 0.86],
	];
	ctx.strokeStyle = "rgba(18,18,22,0.42)";
	ctx.lineWidth = 1.3;
	for (const [, x, y] of spokes) {
		ctx.beginPath();
		ctx.moveTo(hx, hy);
		ctx.lineTo(x, y);
		ctx.stroke();
	}
	ctx.lineWidth = 1;
	graphChip(ctx, app.ui.customer, hx, hy, true, w * 0.5);
	for (const [label, x, y] of spokes) {
		const room = 2 * Math.min(x - 6, w - 6 - x);
		graphChip(ctx, label, x, y, false, room);
	}
}

function controlBody(ctx: Ctx, app: DescentApp, w: number) {
	const rows: [string, string, string][] = [
		[app.ui.row1, app.ui.compliant, T.green],
		[app.ui.row2, app.ui.compliant, T.green],
		[app.ui.row3, app.ui.reviewStatus, T.amber],
	];
	for (const [i, [label, status, col]] of rows.entries()) {
		const y = 47 + i * 48;
		rr(ctx, 12, y, w - 24, 41, 9);
		ctx.fillStyle = T.card;
		ctx.fill();
		ctx.strokeStyle = T.border;
		rr(ctx, 12.5, y + 0.5, w - 25, 40, 9);
		ctx.stroke();
		ctx.beginPath();
		ctx.arc(28, y + 20.5, 4.5, 0, Math.PI * 2);
		ctx.fillStyle = col;
		ctx.fill();
		const sw = text(ctx, status, w - 24, y + 25, {
			size: 11.5,
			weight: 640,
			color: col,
			align: "right",
			max: (w - 24) * 0.4,
		});
		text(ctx, label, 41, y + 25, {
			size: 13,
			weight: 610,
			color: T.fg,
			max: w - 24 - sw - 60,
		});
	}
	const gy = 47 + 3 * 48 + 4;
	rr(ctx, 12, gy, w - 24, 10, 5);
	ctx.fillStyle = T.muted;
	ctx.fill();
	const g = ctx.createLinearGradient(12, 0, w - 12, 0);
	g.addColorStop(0, T.green);
	g.addColorStop(1, T.amber);
	rr(ctx, 12, gy, (w - 24) * 0.82, 10, 5);
	ctx.fillStyle = g;
	ctx.fill();
}

function inventoryBody(ctx: Ctx, _app: DescentApp, w: number) {
	for (const [i, [city, pct, val]] of FIXTURES.inventoryRows.entries()) {
		const y = 46 + i * 40;
		rr(ctx, 12, y, w - 24, 34, 8);
		ctx.fillStyle = T.card;
		ctx.fill();
		text(ctx, city, 23, y + 22, { size: 12.5, weight: 620, color: T.fg });
		const bx = 86;
		const bw = w - bx - 70;
		rr(ctx, bx, y + 14, bw, 7, 4);
		ctx.fillStyle = T.muted;
		ctx.fill();
		rr(ctx, bx, y + 14, (bw * pct) / 100, 7, 4);
		ctx.fillStyle = i === 2 ? emberGradient(ctx, bx, 0, bx + bw, 0) : "#f08a6a";
		ctx.fill();
		text(ctx, val, w - 23, y + 22, {
			size: 12,
			weight: 700,
			color: T.fg,
			align: "right",
			mono: true,
		});
	}
}

function agentOrb(ctx: Ctx, ox: number, oy: number) {
	const g = ctx.createRadialGradient(ox - 5, oy - 6, 2, ox, oy, 18);
	g.addColorStop(0, "#ffb489");
	g.addColorStop(0.45, T.primary);
	g.addColorStop(1, "#a3122a");
	ctx.save();
	ctx.shadowColor = "rgba(251,86,45,0.55)";
	ctx.shadowBlur = 16;
	ctx.beginPath();
	ctx.arc(ox, oy, 17, 0, Math.PI * 2);
	ctx.fillStyle = g;
	ctx.fill();
	ctx.restore();
	text(ctx, "✦", ox, oy + 5, {
		size: 14,
		weight: 600,
		color: "#fff",
		align: "center",
	});
}

function agentChips(ctx: Ctx, chips: readonly string[], w: number, cy: number) {
	const gap = 18;
	const room = (w - 24 - gap * 2) / chips.length;
	const fitted = chips.map((c) =>
		fitLine(measurer(ctx, 620), c, room - 18, {
			size: 11.5,
			floor: 10,
			locale: LOCALE,
		}),
	);
	const widths = fitted.map((f) => measurer(ctx, 620)(f.text, f.size) + 18);
	const total = widths.reduce((a, b) => a + b, 0) + gap * 2;
	let x = (w - total) / 2;
	for (const [i, f] of fitted.entries()) {
		const hot = i === 2;
		rr(ctx, x, cy, widths[i], 28, 8);
		ctx.fillStyle = hot ? "rgba(251,86,45,0.16)" : "rgba(255,255,255,0.05)";
		ctx.fill();
		ctx.strokeStyle = hot ? "rgba(251,86,45,0.55)" : T.dBorder;
		rr(ctx, x + 0.5, cy + 0.5, widths[i] - 1, 27, 8);
		ctx.stroke();
		text(ctx, f.text, x + widths[i] / 2, cy + 18.5, {
			size: f.size,
			weight: 620,
			color: T.dFg,
			align: "center",
		});
		x += widths[i];
		if (i < chips.length - 1) {
			text(ctx, "→", x + gap / 2, cy + 18.5, {
				size: 11.5,
				weight: 500,
				color: T.dMutedFg,
				align: "center",
			});
			x += gap;
		}
	}
}

// The briefing is ready: one wide pill with a check replaces the three inputs.
function agentReady(ctx: Ctx, label: string, w: number, cy: number) {
	rr(ctx, 12, cy, w - 24, 28, 8);
	ctx.fillStyle = "rgba(251,86,45,0.2)";
	ctx.fill();
	ctx.strokeStyle = "rgba(251,86,45,0.62)";
	rr(ctx, 12.5, cy + 0.5, w - 25, 27, 8);
	ctx.stroke();
	check(ctx, 24, cy + 9.5, 9, "#ff8a5c", 1.9);
	text(ctx, label, 40, cy + 18.5, {
		size: 12,
		weight: 650,
		color: "#ffe2d6",
		max: w - 64,
	});
}

function agentBody(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	h: number,
	face: AppFace,
) {
	agentOrb(ctx, 30, 70);
	wrap(
		ctx,
		app.ui.prompt,
		{ x: 58, y: 64, maxW: w - 72, lh: 19, maxLines: 4 },
		{ size: 14, weight: 540, color: T.dFg, min: 12 },
	);
	if (face.state === "arrived")
		agentReady(ctx, face.arrivals.briefingReady, w, h - 52);
	else agentChips(ctx, [app.ui.mail, app.ui.data, app.ui.calendar], w, h - 52);
}

function fieldMap(ctx: Ctx, w: number, my: number, mh: number) {
	ctx.save();
	rr(ctx, 8, my, w - 16, mh, 12);
	ctx.clip();
	ctx.fillStyle = "#ebe4df";
	ctx.fillRect(8, my, w - 16, mh);
	ctx.strokeStyle = "#ffffff";
	ctx.lineCap = "round";
	const roads: [[number, number], [number, number], number][] = [
		[[-10, my + 78], [w + 10, my + 30], 9],
		[[16, my - 10], [w - 30, my + mh + 10], 7],
		[[-10, my + 30], [w + 10, my + 66], 4],
	];
	for (const [a, b, lw] of roads) {
		ctx.lineWidth = lw;
		ctx.beginPath();
		ctx.moveTo(a[0], a[1]);
		ctx.lineTo(b[0], b[1]);
		ctx.stroke();
	}
	ctx.strokeStyle = T.primary;
	ctx.lineWidth = 3;
	ctx.setLineDash([5, 4]);
	ctx.beginPath();
	ctx.moveTo(22, my + 90);
	ctx.quadraticCurveTo(48, my + 64, w * 0.52, my + 52);
	ctx.stroke();
	ctx.setLineDash([]);
	ctx.restore();
	ctx.beginPath();
	ctx.arc(w * 0.55, my + 50, 13, 0, Math.PI * 2);
	ctx.fillStyle = T.fg;
	ctx.fill();
	ctx.lineWidth = 3;
	ctx.strokeStyle = "#fff";
	ctx.stroke();
	text(ctx, FIXTURES.field.stop, w * 0.55, my + 54.5, {
		size: 12,
		weight: 720,
		color: "#fff",
		align: "center",
	});
}

// The phone has no window chrome, so its status floats on the map.
function fieldStatus(ctx: Ctx, status: Status, w: number, my: number) {
	ctx.save();
	softShadow(ctx, 6, 1, 0.12);
	const pill = pillWidth(ctx, status.label, w - 28);
	rr(ctx, w - 14 - pill, my + 7, pill, 20, 10);
	ctx.fillStyle = "#fff";
	ctx.fill();
	ctx.restore();
	statusPill(ctx, status, w - 14, my + 7, false, w - 28);
}

function fieldBody(
	ctx: Ctx,
	app: DescentApp,
	w: number,
	h: number,
	face: AppFace,
) {
	appIcon(ctx, 10, 11, 18, app.name.charAt(0), false);
	text(ctx, app.name, 34, 24.5, {
		size: 12,
		weight: 680,
		color: T.fg,
		max: w - 40,
		min: 10.5,
	});
	const my = 36;
	const mh = 104;
	fieldMap(ctx, w, my, mh);
	const status = statusOf(face);
	if (status.label) fieldStatus(ctx, status, w, my);
	const py = my + mh + 22;
	const muted = { size: 11, weight: 560, color: T.mutedFg, max: w - 20 };
	const scheduled = face.state === "arrived";
	text(ctx, app.ui.nextVisit, 12, py, muted);
	text(
		ctx,
		scheduled ? face.arrivals.scheduled : FIXTURES.field.site,
		12,
		py + 20,
		{
			size: 14.5,
			weight: 700,
			color: scheduled ? "#cf3a1b" : T.fg,
			max: w - 20,
			min: 11,
		},
	);
	text(ctx, FIXTURES.field.eta, 12, py + 38, { ...muted, weight: 520 });
	rr(ctx, 10, h - 46, w - 20, 32, 10);
	ctx.fillStyle = T.fg;
	ctx.fill();
	text(ctx, app.ui.startRoute, w / 2, h - 25.5, {
		size: 12,
		weight: 650,
		color: "#fff",
		align: "center",
		max: w - 32,
	});
}

type BodyDrawer = (
	ctx: Ctx,
	app: DescentApp,
	w: number,
	h: number,
	face: AppFace,
) => void;

const BODY: Record<AppKind, BodyDrawer> = {
	ops: opsBody,
	finance: financeBody,
	customer: customerBody,
	graph: graphBody,
	control: controlBody,
	inventory: inventoryBody,
	agent: agentBody,
	field: fieldBody,
};

/** Where each face changes when work arrives, as UV x0, y0, x1, y1 from the top left. */
export function arrivalRegion(spec: AppSpec): Rect | null {
	const { w, h } = spec;
	const uv = (x0: number, y0: number, x1: number, y1: number): Rect => [
		x0 / w,
		y0 / h,
		x1 / w,
		y1 / h,
	];
	switch (spec.kind) {
		case "ops":
			return uv(12, 47, w - 12, h - 11);
		case "finance":
			return uv(12, 102, w - 12, 150);
		case "agent":
			return uv(12, h - 52, w - 12, h - 24);
		case "field":
			return uv(8, 150, w - 8, 204);
		default:
			return null;
	}
}

const IDLE_FACE: AppFace = {
	state: "idle",
	arrivals: {
		approved: "",
		briefingReady: "",
		scheduled: "",
		live: "",
		ready: "",
	},
};

/** The full window face (chrome + body) at its design size. */
export function drawAppWindow(
	app: DescentApp,
	spec: AppSpec,
	scale = TEX_SCALE,
	face: AppFace = IDLE_FACE,
	target?: HTMLCanvasElement,
) {
	const { w, h } = spec;
	const dark = !!spec.dark;
	const r = spec.phone ? 20 : 13;
	const { c, ctx } = makeCanvas(w, h, scale, target);
	ctx.save();
	rr(ctx, 0, 0, w, h, r);
	ctx.clip();
	ctx.fillStyle = dark ? T.dCard : spec.phone ? T.card : T.body;
	ctx.fillRect(0, 0, w, h);
	if (!spec.phone) {
		ctx.fillStyle = dark ? "#171a20" : T.card;
		ctx.fillRect(0, 0, w, 37);
		header(ctx, app, w, dark, face);
	}
	BODY[spec.kind](ctx, app, w, h, face);
	ctx.restore();
	return { canvas: c, w, h, radius: r };
}
