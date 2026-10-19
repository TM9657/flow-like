import type {
	BoardDirection,
	BoardEdge,
	BoardNode,
	BoardNodeId,
	BoardPin,
	BoardScenario,
	BoardScenarioId,
	BoardSpec,
	BoardThread,
} from "./board-spec";

export type Point = readonly [x: number, y: number];
export type Polyline = readonly Point[];

export interface Rect {
	readonly x: number;
	readonly y: number;
	readonly width: number;
	readonly height: number;
}

/** Width in px of `text` drawn with the CSS `font`. */
export type MeasureText = (text: string, font: string) => number;

export interface BoardLayoutOptions {
	readonly measure: MeasureText;
	/** CSS font-family list of the page's Inter. */
	readonly fontFamily: string;
}

/** Board units match the product's flow coordinates (one unit is one CSS px at zoom 1). */
export const BOARD_METRICS = {
	minWidth: 150,
	maxWidth: 264,
	header: 16,
	firstPin: 28,
	pinRow: 15,
	gutter: 60,
	radius: 4,
	titleSize: 10,
	pinSize: 9.6,
	titleX: 18,
	labelInset: 10,
	labelGap: 12,
	reference: 12,
	referenceClearance: 4,
	curveSegments: 32,
} as const;

const ELLIPSIS = "…";

export interface PlacedPin extends BoardPin {
	/** Centre in node-local board units. */
	readonly x: number;
	readonly y: number;
	/** The label as painted: the full label, or an ellipsized prefix when it cannot fit. */
	readonly text: string;
	/** Clip width of the label, the egui `labelMaxWidth`. */
	readonly labelMaxWidth: number;
}

export interface PlacedNode extends Omit<BoardNode, "pins"> {
	readonly x: number;
	readonly y: number;
	readonly width: number;
	readonly height: number;
	readonly titleText: string;
	readonly titleSize: number;
	readonly pins: readonly PlacedPin[];
}

export interface PlacedEdge extends BoardEdge {
	/** The egui cubic bezier from pin centre to pin centre, 33 samples. */
	readonly points: Polyline;
}

export interface PlacedThread extends BoardThread {
	/** Centre of the node's top reference pin, in board units. */
	readonly anchor: Point;
}

export interface BoardRoute {
	readonly id: BoardScenarioId;
	/** Nodes in the order the route reaches them. */
	readonly nodes: readonly BoardNodeId[];
	/** Edges in travel order. */
	readonly edges: readonly string[];
	readonly threads: readonly string[];
	readonly beats: BoardScenario["beats"];
}

export interface BoardLayout {
	readonly nodes: readonly PlacedNode[];
	readonly edges: readonly PlacedEdge[];
	readonly threads: readonly PlacedThread[];
	/** Everything painted, including pins, reference squares and wires. */
	readonly bounds: Rect;
	readonly routes: Readonly<Record<BoardScenarioId, BoardRoute>>;
	readonly fontFamily: string;
	node(id: BoardNodeId): PlacedNode;
	pinPosition(nodeId: BoardNodeId, pinId: string): Point;
	edgePath(edgeId: string): Polyline;
	threadAnchor(threadId: string): Point;
}

export const titleFont = (size: number, family: string) =>
	`500 ${size}px ${family}`;
export const pinFont = (size: number, family: string) =>
	`400 ${size}px ${family}`;

/** The product's title fitting: titles longer than 30 characters shrink, down to 60%. */
export function titleScale(text: string): number {
	const length = Array.from(text).length;
	return length <= 30 ? 1 : Math.max((30 / length) ** 1.3, 0.6);
}

/** One end of an egui `CurveKey`: reference pins leave and enter vertically. */
export interface CurveEnd {
	readonly at: Point;
	readonly direction: BoardDirection;
	readonly vertical: boolean;
}

const handleOffset = (distance: number) =>
	distance >= 0 ? distance * 0.5 : 6.25 * Math.sqrt(-distance);

const sign = (direction: BoardDirection) => (direction === "input" ? -1 : 1);

/** egui `Curve::new`: handles leave each pin along its axis, at half the distance (6.25·√d backwards). */
export function boardCurve(from: CurveEnd, to: CurveEnd): Polyline {
	const a = from.at;
	const d = to.at;
	const b = [a[0], a[1]];
	const c = [d[0], d[1]];
	const fromAxis = from.vertical ? 1 : 0;
	const toAxis = to.vertical ? 1 : 0;
	b[fromAxis] +=
		sign(from.direction) *
		handleOffset((d[fromAxis] - a[fromAxis]) * sign(from.direction));
	c[toAxis] +=
		sign(to.direction) *
		handleOffset((a[toAxis] - d[toAxis]) * sign(to.direction));
	const segments = BOARD_METRICS.curveSegments;
	return Array.from({ length: segments + 1 }, (_, i): Point => {
		const t = i / segments;
		const u = 1 - t;
		const w = [u * u * u, 3 * u * u * t, 3 * u * t * t, t * t * t];
		return [
			a[0] * w[0] + b[0] * w[1] + c[0] * w[2] + d[0] * w[3],
			a[1] * w[0] + b[1] * w[1] + c[1] * w[2] + d[1] * w[3],
		];
	});
}

/** Longest prefix that fits `max` with a trailing ellipsis, as egui's `truncate_at_width`. */
export function fitText(
	text: string,
	max: number,
	font: string,
	measure: MeasureText,
): string {
	if (measure(text, font) <= max) return text;
	const glyphs = Array.from(text);
	let low = 0;
	let high = glyphs.length - 1;
	while (low < high) {
		const mid = Math.ceil((low + high) / 2);
		if (measure(glyphs.slice(0, mid).join("") + ELLIPSIS, font) <= max)
			low = mid;
		else high = mid - 1;
	}
	return `${glyphs.slice(0, low).join("").trimEnd()}${ELLIPSIS}`;
}

/** Pins that sit in label rows, in row order (reference pins sit on the top or bottom edge). */
const rowPins = (pins: readonly BoardPin[], direction: BoardDirection) =>
	pins.filter((pin) => pin.direction === direction && pin.kind !== "reference");

const rowCount = (node: BoardNode) =>
	Math.max(
		1,
		rowPins(node.pins, "input").length,
		rowPins(node.pins, "output").length,
	);

const nodeHeight = (node: BoardNode) =>
	BOARD_METRICS.firstPin + rowCount(node) * BOARD_METRICS.pinRow;

/** Each label row as [input, output] pins. */
function labelRows(node: BoardNode) {
	const inputs = rowPins(node.pins, "input");
	const outputs = rowPins(node.pins, "output");
	return Array.from(
		{ length: Math.max(inputs.length, outputs.length) },
		(_, row) => [inputs[row], outputs[row]] as const,
	);
}

interface Measured {
	readonly node: BoardNode;
	/** The width this node needs so nothing truncates. */
	readonly need: number;
	readonly titleSize: number;
	readonly title: number;
	readonly labels: ReadonlyMap<string, number>;
}

function measureNode(
	node: BoardNode,
	measure: MeasureText,
	family: string,
): Measured {
	const m = BOARD_METRICS;
	const titleSize = m.titleSize * titleScale(node.title);
	const title = measure(node.title, titleFont(titleSize, family)) + 1;
	const labels = new Map(
		node.pins
			.filter((pin) => pin.label)
			.map((pin) => [
				pin.id,
				measure(pin.label, pinFont(m.pinSize, family)) + 1,
			]),
	);
	const width = (pin: BoardPin | undefined) => labels.get(pin?.id ?? "") ?? 0;
	const needs = [
		m.minWidth,
		title + m.titleX + 6,
		...labelRows(node).map(
			([input, output]) =>
				2 * m.labelInset + m.labelGap + width(input) + width(output),
		),
	];
	if (node.pins.some((pin) => pin.kind === "reference"))
		needs.push(2 * (m.titleX + title + m.reference / 2 + m.referenceClearance));
	return { node, need: Math.max(...needs), titleSize, title, labels };
}

/** Label clip widths of one row: each side its need plus half the spare, or a fair split when crowded. */
function rowCaps(width: number, a: number, b: number): readonly number[] {
	const available =
		width - 2 * BOARD_METRICS.labelInset - BOARD_METRICS.labelGap;
	if (!a || !b) return [available, available];
	if (a + b <= available) {
		const spare = (available - a - b) / 2;
		return [a + spare, b + spare];
	}
	const shorter = Math.min(Math.min(a, b), available / 2);
	return a <= b
		? [shorter, available - shorter]
		: [available - shorter, shorter];
}

function placePins(
	measured: Measured,
	width: number,
	measure: MeasureText,
	family: string,
): PlacedPin[] {
	const m = BOARD_METRICS;
	const font = pinFont(m.pinSize, family);
	const placed = new Map<string, PlacedPin>();
	const labelWidth = (pin: BoardPin | undefined) =>
		measured.labels.get(pin?.id ?? "") ?? 0;
	labelRows(measured.node).forEach(([input, output], row) => {
		const caps = rowCaps(width, labelWidth(input), labelWidth(output));
		[input, output].forEach((pin, side) => {
			if (!pin) return;
			placed.set(pin.id, {
				...pin,
				x: side === 0 ? 0 : width,
				y: m.firstPin + row * m.pinRow,
				text: pin.label ? fitText(pin.label, caps[side], font, measure) : "",
				labelMaxWidth: caps[side],
			});
		});
	});
	for (const pin of measured.node.pins)
		if (pin.kind === "reference")
			placed.set(pin.id, {
				...pin,
				x: width / 2,
				y: pin.direction === "input" ? 0 : nodeHeight(measured.node),
				text: "",
				labelMaxWidth: 0,
			});
	return measured.node.pins.map((pin) => placed.get(pin.id) as PlacedPin);
}

function placeNode(
	measured: Measured,
	at: Point,
	width: number,
	options: BoardLayoutOptions,
): PlacedNode {
	const m = BOARD_METRICS;
	const { node } = measured;
	const titleRoom = node.pins.some((pin) => pin.kind === "reference")
		? Math.min(
				width - 24,
				width / 2 - m.reference / 2 - m.referenceClearance - m.titleX,
			)
		: width - 24;
	return {
		id: node.id,
		kind: node.kind,
		column: node.column,
		row: node.row,
		offset: node.offset,
		title: node.title,
		x: at[0],
		y: at[1] + node.offset * m.pinRow,
		width,
		height: nodeHeight(node),
		titleText: fitText(
			node.title,
			titleRoom,
			titleFont(measured.titleSize, options.fontFamily),
			options.measure,
		),
		titleSize: measured.titleSize,
		pins: placePins(measured, width, options.measure, options.fontFamily),
	};
}

/** Running offsets of cells separated by even gutters. */
const offsets = (sizes: readonly number[]) =>
	sizes.map((_, i) =>
		sizes
			.slice(0, i)
			.reduce((sum, size) => sum + size + BOARD_METRICS.gutter, 0),
	);

const evenCeil = (value: number) => Math.ceil(value / 2) * 2;

/** One shared node width keeps the grid even; rows are as tall as their tallest node. */
function placeNodes(
	spec: BoardSpec,
	options: BoardLayoutOptions,
): PlacedNode[] {
	const m = BOARD_METRICS;
	const measured = spec.nodes.map((node) =>
		measureNode(node, options.measure, options.fontFamily),
	);
	const width = Math.min(
		m.maxWidth,
		evenCeil(Math.max(...measured.map((entry) => entry.need))),
	);
	const columns = Math.max(...spec.nodes.map((node) => node.column)) + 1;
	const rows = Math.max(...spec.nodes.map((node) => node.row)) + 1;
	const x = offsets(Array.from({ length: columns }, () => width));
	const y = offsets(
		Array.from({ length: rows }, (_, row) =>
			Math.max(
				0,
				...spec.nodes
					.filter((node) => node.row === row)
					.map((node) => node.offset * m.pinRow + nodeHeight(node)),
			),
		),
	);
	return measured.map((entry) =>
		placeNode(entry, [x[entry.node.column], y[entry.node.row]], width, options),
	);
}

function lookup<T extends { readonly id: string }>(
	items: readonly T[],
	what: string,
) {
	const byId = new Map(items.map((item) => [item.id, item]));
	return (id: string): T => {
		const found = byId.get(id);
		if (!found) throw new Error(`Unknown board ${what} "${id}"`);
		return found;
	};
}

function union(rects: readonly Rect[]): Rect {
	const x = Math.min(...rects.map((r) => r.x));
	const y = Math.min(...rects.map((r) => r.y));
	const right = Math.max(...rects.map((r) => r.x + r.width));
	const bottom = Math.max(...rects.map((r) => r.y + r.height));
	return { x, y, width: right - x, height: bottom - y };
}

const pointsRect = (points: Polyline): Rect =>
	union(points.map(([x, y]) => ({ x, y, width: 0, height: 0 })));

function boardBounds(
	nodes: readonly PlacedNode[],
	edges: readonly PlacedEdge[],
): Rect {
	const pad = BOARD_METRICS.reference / 2;
	return union([
		...nodes.map((node) => ({
			x: node.x - pad,
			y: node.y - pad,
			width: node.width + 2 * pad,
			height: node.height + 2 * pad,
		})),
		...edges.map((edge) => {
			const r = pointsRect(edge.points);
			return {
				x: r.x - 2,
				y: r.y - 2,
				width: r.width + 4,
				height: r.height + 4,
			};
		}),
	]);
}

function buildRoute(
	scenario: BoardScenario,
	edge: (id: string) => PlacedEdge,
	thread: (id: string) => PlacedThread,
): BoardRoute {
	const nodes = new Set<BoardNodeId>();
	const threads = new Set<string>();
	const edges: string[] = [];
	for (const beat of scenario.beats) {
		for (const hop of beat.threads) {
			nodes.add(thread(hop.id).node);
			threads.add(hop.id);
		}
		for (const id of beat.edges) {
			const { source, target } = edge(id);
			nodes.add(source.node).add(target.node);
			edges.push(id);
		}
	}
	return {
		id: scenario.id,
		nodes: [...nodes],
		edges,
		threads: [...threads],
		beats: scenario.beats,
	};
}

/** Sizes and places the board like the product: nodes are 150 wide unless a title or label needs more. */
export function layoutBoard(
	spec: BoardSpec,
	options: BoardLayoutOptions,
): BoardLayout {
	const nodes = placeNodes(spec, options);
	const node: (id: BoardNodeId) => PlacedNode = lookup(nodes, "node");
	const pin = (nodeId: BoardNodeId, pinId: string) =>
		lookup(node(nodeId).pins, `pin on node "${nodeId}"`)(pinId);
	const pinPosition = (nodeId: BoardNodeId, pinId: string): Point => {
		const owner = node(nodeId);
		const found = pin(nodeId, pinId);
		return [owner.x + found.x, owner.y + found.y];
	};
	const curveEnd = ({ node: id, pin: pinId }: BoardEdge["source"]) => {
		const found = pin(id, pinId);
		return {
			at: pinPosition(id, pinId),
			direction: found.direction,
			vertical: found.kind === "reference",
		};
	};
	const edges = spec.edges.map(
		(edge): PlacedEdge => ({
			...edge,
			points: boardCurve(curveEnd(edge.source), curveEnd(edge.target)),
		}),
	);
	const threads = spec.threads.map(
		(thread): PlacedThread => ({
			...thread,
			anchor: pinPosition(thread.node, thread.pin),
		}),
	);
	const edge = lookup(edges, "edge");
	const thread = lookup(threads, "thread");
	return {
		nodes,
		edges,
		threads,
		bounds: boardBounds(nodes, edges),
		routes: Object.fromEntries(
			spec.scenarios.map((scenario) => [
				scenario.id,
				buildRoute(scenario, edge, thread),
			]),
		) as Record<BoardScenarioId, BoardRoute>,
		fontFamily: options.fontFamily,
		node,
		pinPosition,
		edgePath: (edgeId) => edge(edgeId).points,
		threadAnchor: (threadId) => thread(threadId).anchor,
	};
}
