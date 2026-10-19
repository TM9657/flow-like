import {
	BOARD_METRICS,
	type BoardLayout,
	type PlacedEdge,
	type PlacedNode,
	type PlacedPin,
	type Point,
	pinFont,
	titleFont,
} from "./board-layout";
import type {
	BoardDataType,
	BoardNodeId,
	BoardNodeKind,
	BoardPinKind,
} from "./board-spec";

export type RGBA = readonly [number, number, number, number];
export type BoardContext =
	| CanvasRenderingContext2D
	| OffscreenCanvasRenderingContext2D;

export interface BoardPalette {
	readonly background: RGBA;
	readonly card: RGBA;
	readonly foreground: RGBA;
	readonly muted: RGBA;
	readonly border: RGBA;
	readonly primary: RGBA;
	readonly tertiary: RGBA;
	/** Header tint of function nodes. */
	readonly function: RGBA;
	/** `--pin-fn-ref`, for reference pins and wires. */
	readonly reference: RGBA;
	/** `--pin-<type>` colours of data pins and wires. */
	readonly types: Readonly<Record<BoardDataType, RGBA>>;
}

/** The product's dark theme tokens, resolved to bytes as `readCanvasTheme` does. */
export const BOARD_PALETTE_DARK: BoardPalette = {
	background: [12, 14, 19, 255],
	card: [19, 22, 27, 255],
	foreground: [243, 239, 236, 255],
	muted: [153, 158, 168, 255],
	border: [40, 44, 51, 255],
	primary: [251, 86, 45, 255],
	tertiary: [241, 151, 48, 255],
	function: [139, 92, 246, 255],
	reference: [225, 91, 13, 255],
	types: { Struct: [152, 16, 250, 255], String: [251, 100, 182, 255] },
};

/** Maps board units to canvas units: `canvas = view.x/y + board * view.zoom`, like egui's viewport. */
export interface BoardView {
	readonly x: number;
	readonly y: number;
	readonly zoom: number;
}

/**
 * How far the board steps back behind its active route, for a board that
 * supports a page rather than being worked on: unset, it dims like egui.
 */
export interface BoardEmphasis {
	/** Opacity of every node and wire off the active route (egui dims nodes to 0.45 and keeps wires); 0 leaves them out. */
	readonly inactive: number;
	/** Opacity of all titles and labels, so text reads as texture rather than copy. */
	readonly text: number;
	/** Opacity of the wires off the active route, when it differs from `inactive`; 0 leaves only the route's wires. */
	readonly wires?: number;
}

export interface BoardPaintState {
	readonly view?: BoardView;
	/** When set, these nodes get the selected border and every other node is dimmed. */
	readonly activeNodes?: ReadonlySet<BoardNodeId> | null;
	/** Edges painted with the selected stroke; the rest keep their normal alpha. */
	readonly activeEdges?: ReadonlySet<string> | null;
	readonly emphasis?: BoardEmphasis | null;
}

/** One stroke of a wire as `Board::paint` draws it: [normal, selected] width and alpha. */
interface StrokeSpec {
	readonly width: readonly [number, number];
	readonly alpha: readonly [number, number];
	readonly lighten?: number;
	readonly dashed?: boolean;
}

const EDGE_STROKES: Readonly<Record<BoardPinKind, readonly StrokeSpec[]>> = {
	execution: [{ width: [2, 3], alpha: [0.7, 0.9] }],
	reference: [
		{ width: [5, 8], alpha: [0.04, 0.08] },
		{ width: [1.5, 2.5], alpha: [0.3, 0.5] },
		{ width: [0.8, 1.2], alpha: [0.4, 0.7], lighten: 0.2 },
	],
	data: [
		{ width: [1.5, 2.5], alpha: [0.35, 0.5], dashed: true },
		{ width: [1, 1.5], alpha: [0.5, 0.8], lighten: 0.3, dashed: true },
	],
};

const HEADER_TINT: Readonly<
	Record<BoardNodeKind, (palette: BoardPalette) => RGBA>
> = {
	event: (palette) => palette.primary,
	function: (palette) => palette.function,
	pure: (palette) => palette.tertiary,
	standard: (palette) => palette.card,
};

const WHITE: RGBA = [255, 255, 255, 255];
const DIMMED = 0.45;
const DIAMOND = 5.657;
const DOT = 3.5;
/**
 * Device px from which `geometricPrecision` glyphs match egui's ink within ~7%; Chrome's
 * default macOS glyphs carry ~15% more, but stay closer for small hinted text.
 */
const HINTED_TEXT_PX = 24;
const INTER_ASCENT = 1984 / 2048;
const INTER_DESCENT = 494 / 2048;
const HEADER_ARC: readonly Point[] = [
	[1, 0],
	[0.9238795, -0.3826834],
	[Math.SQRT1_2, -Math.SQRT1_2],
	[0.3826834, -0.9238795],
	[0, -1],
];
/** The header's two-square workflow glyph, in node-local units. */
const GLYPH_LINK: readonly Point[] = [
	[8.33, 7.67],
	[8.33, 9],
	[9, 9.67],
	[10.33, 9.67],
];
const GLYPH_SQUARES: readonly Point[] = [
	[7, 5],
	[10.33, 8.33],
];

export function mix(a: RGBA, b: RGBA, amount: number): RGBA {
	const channel = (i: number) =>
		Math.floor(a[i] * (1 - amount) + b[i] * amount);
	return [channel(0), channel(1), channel(2), channel(3)];
}

export function css(color: RGBA, opacity = 1): string {
	return `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${Math.floor(color[3] * opacity) / 255})`;
}

export function pinColor(
	kind: BoardPinKind,
	type: string,
	palette: BoardPalette,
): RGBA {
	if (kind === "execution") return palette.foreground;
	if (kind === "reference") return palette.reference;
	return palette.types[type as BoardDataType] ?? palette.foreground;
}

/** Exec wires use the foreground, data wires their source pin's type colour. */
export function edgeColor(edge: PlacedEdge, palette: BoardPalette): RGBA {
	return pinColor(edge.kind, edge.type, palette);
}

/** egui `header_outline`: a convex header with rounded top corners and a square bottom edge. */
function headerOutline(width: number, height: number, radius: number) {
	const r = Math.min(Math.max(radius, 0), width * 0.5, height);
	const points: Point[] = [
		[0, height],
		[width, height],
	];
	if (r === 0) {
		points.push([width, 0], [0, 0]);
		return points;
	}
	const clamp = (value: number, max: number) =>
		Math.min(Math.max(value, 0), max);
	for (const left of [false, true]) {
		const center = left ? r : width - r;
		for (const [ax, ay] of HEADER_ARC) {
			const [x, y] = left ? [ay, -ax] : [ax, ay];
			points.push([clamp(center + r * x, width), clamp(r + r * y, height)]);
		}
	}
	return points;
}

/** egui `dashed_line`: dashes restart at every vertex they span, so each piece is its own segment. */
function dashSegments(points: readonly Point[], dash: number, gap: number) {
	const out: [Point, Point][] = [];
	let position = 0;
	let drawing = false;
	for (let i = 0; i + 1 < points.length; i++) {
		const start = points[i];
		const end = points[i + 1];
		const dx = end[0] - start[0];
		const dy = end[1] - start[1];
		const length = Math.hypot(dx, dy);
		let from = start;
		while (position < length) {
			const at: Point = [
				start[0] + (dx * position) / length,
				start[1] + (dy * position) / length,
			];
			if (drawing) out.push([from, at]);
			else from = at;
			position += drawing ? gap : dash;
			drawing = !drawing;
		}
		if (drawing) out.push([from, end]);
		position -= length;
	}
	return out;
}

/** egui `CornerRadius` is whole points, and the tessellator adds 0.4 to every rounded corner. */
const cornerRadius = (radius: number) =>
	radius > 0 ? Math.round(radius) + 0.4 : 0;

/**
 * The device-pixel grid under the context transform. egui snaps rects, axis-aligned segments
 * and text galleys to physical pixels and feathers every stroke end by one pixel.
 */
interface PixelGrid {
	/** Device pixels per canvas unit, egui's `pixels_per_point`. */
	readonly ppp: number;
	readonly origin: Point;
	readonly aligned: boolean;
}

function pixelGrid(ctx: BoardContext): PixelGrid {
	const m = ctx.getTransform();
	const aligned = m.b === 0 && m.c === 0 && m.a > 0 && m.a === m.d;
	const ppp = aligned ? m.a : Math.sqrt(Math.abs(m.a * m.d - m.b * m.c));
	return { ppp: ppp || 1, origin: [m.e, m.f], aligned };
}

/** Rust's `f32::round`: halves away from zero. */
const rustRound = (value: number) =>
	Math.sign(value) * Math.round(Math.abs(value));

/** egui `round_to_pixels`, or `round_to_pixel_center` when `center`. */
function snap(grid: PixelGrid, value: number, axis: 0 | 1, center = false) {
	if (!grid.aligned) return value;
	const device = value * grid.ppp + grid.origin[axis];
	const rounded = center ? rustRound(device - 0.5) + 0.5 : rustRound(device);
	return (rounded - grid.origin[axis]) / grid.ppp;
}

/** egui `Stroke::round_center_to_pixel`: odd or sub-pixel widths sit on pixel centres. */
function oddStroke(grid: PixelGrid, width: number) {
	const pixels = width * grid.ppp;
	return pixels <= 1 || (pixels * 0.5 + 0.25) % 1 > 0.5;
}

/** Extends both ends by half a device pixel, the coverage of egui's one-pixel feathered cap. */
function capEnds(grid: PixelGrid, path: readonly Point[], width: number) {
	if (width * grid.ppp <= 0.9 || path.length < 2) return path;
	const extend = (end: Point, toward: Point): Point => {
		const length = Math.hypot(end[0] - toward[0], end[1] - toward[1]);
		const k = length > 0 ? 0.5 / grid.ppp / length : 0;
		return [
			end[0] + (end[0] - toward[0]) * k,
			end[1] + (end[1] - toward[1]) * k,
		];
	};
	const last = path.length - 1;
	return [
		extend(path[0], path[1]),
		...path.slice(1, last),
		extend(path[last], path[last - 1]),
	];
}

/** egui `tessellate_line_segment`: axis-aligned segments snap to the pixel grid before capping. */
function lineSegment(
	grid: PixelGrid,
	[a, b]: readonly [Point, Point],
	width: number,
): readonly Point[] | null {
	const p = [a[0], a[1]];
	const q = [b[0], b[1]];
	const quarter = 0.25 / grid.ppp;
	for (const axis of [0, 1] as const) {
		if (!grid.aligned || p[axis] !== q[axis]) continue;
		const along = axis === 0 ? 1 : 0;
		const sign = p[along] < q[along] ? 1 : -1;
		p[axis] = q[axis] = snap(grid, p[axis], axis, oddStroke(grid, width));
		p[along] = snap(grid, p[along] + sign * quarter, along, true);
		q[along] = snap(grid, q[along] - sign * quarter, along, true);
	}
	if (p[0] === q[0] && p[1] === q[1]) return null;
	return capEnds(
		grid,
		[
			[p[0], p[1]],
			[q[0], q[1]],
		],
		width,
	);
}

function strokePaths(
	ctx: BoardContext,
	paths: readonly (readonly Point[])[],
	width: number,
	color: string,
) {
	ctx.beginPath();
	for (const path of paths) {
		ctx.moveTo(path[0][0], path[0][1]);
		for (let i = 1; i < path.length; i++) ctx.lineTo(path[i][0], path[i][1]);
	}
	ctx.lineWidth = width;
	ctx.strokeStyle = color;
	ctx.stroke();
}

/** Every dash is its own egui shape, so overlapping caps blend twice as they do on the GPU. */
function strokeDashes(
	ctx: BoardContext,
	grid: PixelGrid,
	points: readonly Point[],
	zoom: number,
	width: number,
	color: string,
) {
	for (const dash of dashSegments(points, 1.5 * zoom, 4 * zoom)) {
		const segment = lineSegment(grid, dash, width);
		if (segment) strokePaths(ctx, [segment], width, color);
	}
}

/** A wire's emphasis: the selected stroke, and the opacity it is painted at. */
interface EdgeLook {
	readonly selected: boolean;
	readonly opacity: number;
}

function paintEdge(
	ctx: BoardContext,
	grid: PixelGrid,
	edge: PlacedEdge,
	base: RGBA,
	view: BoardView,
	look: EdgeLook,
) {
	const z = view.zoom;
	const points = edge.points.map(
		([x, y]): Point => [view.x + x * z, view.y + y * z],
	);
	const state = look.selected ? 1 : 0;
	for (const spec of EDGE_STROKES[edge.kind]) {
		const color = css(
			spec.lighten ? mix(base, WHITE, spec.lighten) : base,
			spec.alpha[state] * look.opacity,
		);
		const width = spec.width[state] * z;
		if (spec.dashed) strokeDashes(ctx, grid, points, z, width, color);
		else strokePaths(ctx, [capEnds(grid, points, width)], width, color);
	}
}

function polygon(ctx: BoardContext, points: readonly Point[]) {
	ctx.beginPath();
	ctx.moveTo(points[0][0], points[0][1]);
	for (let i = 1; i < points.length; i++)
		ctx.lineTo(points[i][0], points[i][1]);
	ctx.closePath();
}

interface Box {
	readonly x: number;
	readonly y: number;
	readonly width: number;
	readonly height: number;
}

/** egui `Rect::round_to_pixels` (or `round_to_pixel_center`) on every edge. */
function snapBox(grid: PixelGrid, box: Box, center = false): Box {
	const x = snap(grid, box.x, 0, center);
	const y = snap(grid, box.y, 1, center);
	return {
		x,
		y,
		width: snap(grid, box.x + box.width, 0, center) - x,
		height: snap(grid, box.y + box.height, 1, center) - y,
	};
}

function roundRect(ctx: BoardContext, box: Box, radius: number) {
	ctx.beginPath();
	ctx.roundRect(
		box.x,
		box.y,
		box.width,
		box.height,
		Math.max(0, Math.min(radius, box.width / 2, box.height / 2)),
	);
}

const insetBox = (box: Box, by: number): Box => ({
	x: box.x + by,
	y: box.y + by,
	width: box.width - 2 * by,
	height: box.height - 2 * by,
});

/** egui `StrokeKind::Inside`: the stroke's outer edge is the box, its corner radius `radius`. */
function strokeInside(
	ctx: BoardContext,
	box: Box,
	radius: number,
	lineWidth: number,
	color: string,
) {
	roundRect(ctx, insetBox(box, lineWidth / 2), radius - lineWidth / 2);
	ctx.lineWidth = lineWidth;
	ctx.strokeStyle = color;
	ctx.stroke();
}

interface PinStyle {
	readonly z: number;
	readonly base: RGBA;
	readonly opacity: number;
	readonly grid: PixelGrid;
}

function paintReferencePin(
	ctx: BoardContext,
	[cx, cy]: Point,
	{ z, base, opacity, grid }: PinStyle,
) {
	const square = (side: number): Box =>
		snapBox(grid, {
			x: cx - (side / 2) * z,
			y: cy - (side / 2) * z,
			width: side * z,
			height: side * z,
		});
	roundRect(ctx, square(12), cornerRadius(2 * z));
	ctx.fillStyle = css(base, opacity);
	ctx.fill();
	strokeInside(
		ctx,
		square(10),
		cornerRadius(z),
		z,
		css(mix(base, WHITE, 0.12), opacity),
	);
}

/** egui's glossy pins are a fan from a light apex up-left of the centre to the pin colour on the rim. */
function paintDataPin(
	ctx: BoardContext,
	[cx, cy]: Point,
	{ z, base, opacity }: PinStyle,
) {
	const radius = DOT * z;
	const tint = css(base, opacity);
	// A focal radial gradient is the homothety of the rim about the apex: the fan's own interpolation.
	const gradient = ctx.createRadialGradient(
		cx - radius * 0.3,
		cy - radius * 0.3,
		0,
		cx,
		cy,
		radius,
	);
	gradient.addColorStop(0, css(mix(base, WHITE, 0.2), opacity));
	gradient.addColorStop(1, tint);
	polygon(
		ctx,
		Array.from({ length: 20 }, (_, i): Point => {
			const angle = (i * Math.PI) / 10 - Math.PI / 2;
			return [cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius];
		}),
	);
	ctx.fillStyle = gradient;
	ctx.fill();
	ctx.beginPath();
	ctx.arc(cx, cy, radius + 0.3 * z, 0, Math.PI * 2);
	ctx.lineWidth = 0.6 * z;
	ctx.strokeStyle = tint;
	ctx.stroke();
}

function paintExecutionPin(
	ctx: BoardContext,
	[cx, cy]: Point,
	{ z, base, opacity }: PinStyle,
) {
	const radius = DIAMOND * z;
	const tint = css(base, opacity);
	const light = css(mix(base, WHITE, 0.2), opacity);
	const apex: Point = [cx - radius * 0.3, cy - radius * 0.3];
	const rim: Point[] = [
		[cx, cy - radius],
		[cx + radius, cy],
		[cx, cy + radius],
		[cx - radius, cy],
	];
	if (opacity >= 1) {
		polygon(ctx, rim);
		ctx.fillStyle = tint;
		ctx.fill();
	}
	rim.forEach((a, i) => {
		const b = rim[(i + 1) % rim.length];
		const ex = b[0] - a[0];
		const ey = b[1] - a[1];
		const t =
			((apex[0] - a[0]) * ex + (apex[1] - a[1]) * ey) / (ex * ex + ey * ey);
		const gradient = ctx.createLinearGradient(
			apex[0],
			apex[1],
			a[0] + ex * t,
			a[1] + ey * t,
		);
		gradient.addColorStop(0, light);
		gradient.addColorStop(1, tint);
		polygon(ctx, [apex, a, b]);
		ctx.fillStyle = gradient;
		ctx.fill();
	});
}

const PIN_PAINTERS: Readonly<
	Record<BoardPinKind, (ctx: BoardContext, at: Point, style: PinStyle) => void>
> = {
	reference: paintReferencePin,
	data: paintDataPin,
	execution: paintExecutionPin,
};

interface Line {
	readonly value: string;
	readonly size: number;
	/** The galley's left edge, or its right edge when `alignRight`. */
	readonly x: number;
	readonly centerY: number;
	readonly alignRight: boolean;
	readonly clip: Box;
}

/** egui's single-row galley: Inter's ascent and row height, rounded the way `galley_from_rows` does. */
function galleyMetrics(size: number, grid: PixelGrid) {
	const ui = (value: number) => rustRound(value * 32) / 32;
	const pixels = (value: number) => rustRound(value * grid.ppp) / grid.ppp;
	const ascent = ui(size * INTER_ASCENT);
	const height = ascent - ui(-size * INTER_DESCENT);
	const rowHeight = pixels(height);
	return { rowHeight, baseline: pixels(ascent + rowHeight - height) };
}

/** One galley centred on `centerY` as egui places it: top-left snapped to a device pixel. */
function text(ctx: BoardContext, grid: PixelGrid, line: Line, color: string) {
	if (!line.value || line.clip.width <= 0) return;
	const { rowHeight, baseline } = galleyMetrics(line.size, grid);
	ctx.save();
	ctx.beginPath();
	ctx.rect(line.clip.x, line.clip.y, line.clip.width, line.clip.height);
	ctx.clip();
	ctx.textRendering =
		line.size * grid.ppp >= HINTED_TEXT_PX ? "geometricPrecision" : "auto";
	ctx.textAlign = "left";
	const width = line.alignRight ? ctx.measureText(line.value).width : 0;
	const top = snap(grid, line.centerY - rowHeight / 2, 1);
	ctx.fillStyle = color;
	ctx.fillText(line.value, snap(grid, line.x - width, 0), top + baseline);
	ctx.restore();
}

/** A node placed on the canvas: node-local board units to canvas units, plus its emphasis. */
interface NodeFrame {
	readonly node: PlacedNode;
	readonly z: number;
	readonly grid: PixelGrid;
	readonly opacity: number;
	readonly foreground: string;
	readonly selected: boolean;
	point(local: Point): Point;
	box(left: number, top: number, right: number, bottom: number): Box;
}

function nodeFrame(
	node: PlacedNode,
	view: BoardView,
	grid: PixelGrid,
	palette: BoardPalette,
	active: boolean | null,
	emphasis: BoardEmphasis | null,
): NodeFrame {
	const z = view.zoom;
	const dimmed = active === false;
	const opacity = dimmed ? (emphasis?.inactive ?? DIMMED) : 1;
	const ink = opacity * (emphasis?.text ?? 1);
	const point = ([x, y]: Point): Point => [
		view.x + (node.x + x) * z,
		view.y + (node.y + y) * z,
	];
	return {
		node,
		z,
		grid,
		opacity,
		selected: active === true,
		foreground: css(dimmed ? palette.muted : palette.foreground, ink),
		point,
		box: (left, top, right, bottom) => {
			const [x, y] = point([left, top]);
			const [x2, y2] = point([right, bottom]);
			return { x, y, width: x2 - x, height: y2 - y };
		},
	};
}

function paintCard(ctx: BoardContext, f: NodeFrame, palette: BoardPalette) {
	const body = snapBox(f.grid, f.box(0, 0, f.node.width, f.node.height));
	const radius = cornerRadius(BOARD_METRICS.radius * f.z);
	const border = (f.selected ? 1.5 : 1) * f.z;
	const inset = f.opacity < 1 ? border : 0;
	roundRect(ctx, insetBox(body, inset), radius - inset);
	ctx.fillStyle = css(palette.card, f.opacity);
	ctx.fill();
	strokeInside(
		ctx,
		body,
		radius,
		border,
		css(f.selected ? palette.primary : palette.border, f.opacity),
	);
}

function paintHeader(ctx: BoardContext, f: NodeFrame, palette: BoardPalette) {
	const m = BOARD_METRICS;
	const width = Math.max(f.node.width - 2, 0);
	polygon(
		ctx,
		headerOutline(
			width,
			Math.max(m.header - 1, 0),
			Math.max(m.radius - 1, 0),
		).map(([x, y]) => f.point([x + 1, y + 1])),
	);
	const [left] = f.point([1, 0]);
	const [right] = f.point([1 + width, 0]);
	const gradient = ctx.createLinearGradient(left, 0, right, 0);
	gradient.addColorStop(0, css(palette.card, f.opacity));
	gradient.addColorStop(1, css(HEADER_TINT[f.node.kind](palette), f.opacity));
	ctx.fillStyle = gradient;
	ctx.fill();
	const separator = lineSegment(
		f.grid,
		[f.point([1, m.header]), f.point([f.node.width - 1, m.header])],
		f.z,
	);
	if (separator)
		strokePaths(ctx, [separator], f.z, css(palette.border, f.opacity));
}

function paintGlyph(ctx: BoardContext, f: NodeFrame) {
	const width = 0.5 * f.z;
	strokePaths(
		ctx,
		[capEnds(f.grid, GLYPH_LINK.map(f.point), width)],
		width,
		f.foreground,
	);
	for (const [gx, gy] of GLYPH_SQUARES) {
		roundRect(
			ctx,
			snapBox(
				f.grid,
				f.box(gx, gy, gx + 2.67, gy + 2.67),
				oddStroke(f.grid, width),
			),
			cornerRadius(0.67 * f.z),
		);
		ctx.lineWidth = width;
		ctx.strokeStyle = f.foreground;
		ctx.stroke();
	}
}

function paintTitle(ctx: BoardContext, f: NodeFrame, family: string) {
	const m = BOARD_METRICS;
	const size = f.node.titleSize * f.z;
	const [x, centerY] = f.point([m.titleX, m.header / 2]);
	ctx.font = titleFont(size, family);
	text(
		ctx,
		f.grid,
		{
			value: f.node.titleText,
			size,
			x,
			centerY,
			alignRight: false,
			clip: f.box(m.titleX, 0, Math.max(f.node.width - 6, m.titleX), m.header),
		},
		f.foreground,
	);
	paintGlyph(ctx, f);
}

function pinLabel(f: NodeFrame, pin: PlacedPin): Line {
	const inset = BOARD_METRICS.labelInset;
	const input = pin.direction === "input";
	const anchor = input ? pin.x + inset : pin.x - inset;
	const left = input ? anchor : anchor - pin.labelMaxWidth;
	const right = input ? anchor + pin.labelMaxWidth : anchor;
	const [x, centerY] = f.point([anchor, pin.y]);
	return {
		value: pin.text,
		size: BOARD_METRICS.pinSize * f.z,
		x,
		centerY,
		alignRight: !input,
		clip: f.box(
			Math.max(left, 0),
			pin.y - 8,
			Math.min(right, f.node.width),
			pin.y + 8,
		),
	};
}

function paintPins(
	ctx: BoardContext,
	f: NodeFrame,
	palette: BoardPalette,
	family: string,
) {
	ctx.font = pinFont(BOARD_METRICS.pinSize * f.z, family);
	for (const pin of f.node.pins) {
		PIN_PAINTERS[pin.kind](ctx, f.point([pin.x, pin.y]), {
			z: f.z,
			base: pinColor(pin.kind, pin.type, palette),
			opacity: f.opacity,
			grid: f.grid,
		});
		text(ctx, f.grid, pinLabel(f, pin), f.foreground);
	}
}

/** A wire's look: the selected stroke on the route, the receding opacity off it. */
function edgeLook(edge: PlacedEdge, state: BoardPaintState): EdgeLook {
	const selected = state.activeEdges?.has(edge.id) ?? false;
	const emphasis = state.emphasis;
	const recede = emphasis && state.activeEdges && !selected;
	return {
		selected,
		opacity: recede ? (emphasis.wires ?? emphasis.inactive) : 1,
	};
}

/** Whether a node is on the route (true), off it (false), or there is no route (null). */
const nodeActive = (node: PlacedNode, state: BoardPaintState) =>
	state.activeNodes ? state.activeNodes.has(node.id) : null;

/** Paints the board like the product's egui canvas: every wire first, then every node on top. */
export function paintBoard(
	ctx: BoardContext,
	layout: BoardLayout,
	palette: BoardPalette,
	state: BoardPaintState = {},
): void {
	const view = state.view ?? { x: 0, y: 0, zoom: 1 };
	ctx.save();
	const grid = pixelGrid(ctx);
	ctx.lineCap = "butt";
	ctx.lineJoin = "round";
	ctx.fontKerning = "normal";
	ctx.textBaseline = "alphabetic";
	for (const edge of layout.edges) {
		const look = edgeLook(edge, state);
		if (look.opacity <= 0) continue;
		paintEdge(ctx, grid, edge, edgeColor(edge, palette), view, look);
	}
	const emphasis = state.emphasis ?? null;
	for (const node of layout.nodes) {
		const active = nodeActive(node, state);
		const f = nodeFrame(node, view, grid, palette, active, emphasis);
		if (f.opacity <= 0) continue;
		paintCard(ctx, f, palette);
		paintHeader(ctx, f, palette);
		paintTitle(ctx, f, layout.fontFamily);
		paintPins(ctx, f, palette, layout.fontFamily);
	}
	ctx.restore();
}
