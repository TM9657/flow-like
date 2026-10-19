import { BOARD_METRICS, type BoardLayout } from "../board/board-layout";
import type { ScenarioPlan } from "./beneath-plan";

/** The follow looks this far ahead, and goes back to the first beat this long before a replay. */
const LEAD = 0.25;
const PREROLL = 0.7;
/** Board units a landing point keeps from a window's edge to count as inside it. */
const INSET = 8;

/** Whole columns a frame of `width` board units holds, and where such a frame can sit. */
export interface ColumnWindows {
	/** Board units from the first column's left edge to the last column's right edge. */
	readonly span: number;
	/** Possible frame centres, one per first column. */
	readonly centers: readonly number[];
}

export function columnWindows(
	layout: BoardLayout,
	width: number,
): ColumnWindows {
	const columns = [...new Set(layout.nodes.map((node) => node.x))].sort(
		(a, b) => a - b,
	);
	const nodeWidth = layout.nodes[0]?.width ?? BOARD_METRICS.minWidth;
	const pitch = nodeWidth + BOARD_METRICS.gutter;
	const count = Math.min(
		columns.length,
		Math.max(1, Math.floor((width + BOARD_METRICS.gutter) / pitch)),
	);
	const span = count * nodeWidth + (count - 1) * BOARD_METRICS.gutter;
	return {
		span,
		centers: columns
			.slice(0, columns.length - count + 1)
			.map((x) => x + span / 2),
	};
}

/** Whether any frame centre holds `point` within `reach`. */
function holdsAny(frames: ColumnWindows, point: number, reach: number) {
	for (const c of frames.centers) {
		if (Math.abs(point - c) <= reach) return true;
	}
	return false;
}

/** The centre within `reach` of `point` nearest `from` (NaN when there is none). */
function nearestCentre(
	frames: ColumnWindows,
	from: number,
	point: number,
	reach: number,
) {
	let best = Number.NaN;
	for (const c of frames.centers) {
		if (Math.abs(point - c) > reach) continue;
		if (Number.isNaN(best) || Math.abs(c - from) < Math.abs(best - from)) {
			best = c;
		}
	}
	return best;
}

/**
 * The frame centre for a landing point: among the frames that hold it, the one
 * nearest the current centre (so the camera stays put while it can), else the
 * frame nearest the point.
 */
export function pickWindow(
	frames: ColumnWindows,
	point: number,
	current: number,
): number {
	const from = Number.isNaN(current) ? point : current;
	const reach = frames.span / 2 - INSET;
	const holding = holdsAny(frames, point, reach);
	return nearestCentre(
		frames,
		from,
		point,
		holding ? reach : Number.POSITIVE_INFINITY,
	);
}

/** The beat a narrow frame follows at `local` seconds into a run; the still shows the last. */
export function followBeat(
	plan: ScenarioPlan,
	local: number,
	moving: boolean,
): number {
	const beats = plan.beats;
	if (!moving) return beats.length - 1;
	if (local > plan.period - PREROLL) return 0;
	let beat = 0;
	for (let i = 0; i < beats.length; i++) {
		if (beats[i].start <= local + LEAD) beat = i;
	}
	return beat;
}

/** How many of the route's nodes lie whole inside the frame centred at `centre`. */
function wholeIn(frames: ColumnWindows, centre: number, plan: ScenarioPlan) {
	const half = frames.span / 2 + 1;
	let held = 0;
	for (const node of plan.nodes) {
		if (node.x >= centre - half && node.x + node.width <= centre + half) held++;
	}
	return held;
}

/**
 * The frame a run rests in: among the frames that hold its landing point, the
 * one that holds the most of the route whole, so the end state shows as much
 * of the story as fits (the one nearest `current` on a tie).
 */
export function restWindow(
	frames: ColumnWindows,
	plan: ScenarioPlan,
	current: number,
): number {
	const landing = plan.beats[plan.beats.length - 1].focus;
	const reach = frames.span / 2 - INSET;
	let best = Number.NaN;
	let most = -1;
	for (const c of frames.centers) {
		if (Math.abs(landing - c) > reach) continue;
		const held = wholeIn(frames, c, plan);
		const nearer = Math.abs(c - current) < Math.abs(best - current);
		if (held > most || (held === most && nearer)) {
			most = held;
			best = c;
		}
	}
	return Number.isNaN(best) ? pickWindow(frames, landing, current) : best;
}

/**
 * The frame centre for a run up to `beat`: each beat in order keeps the previous
 * frame while it still holds that beat's landing point, and the last beat rests
 * where the most of the story shows. Taking the beats from the start of the run
 * makes the frame independent of what was shown before.
 */
export function followWindow(
	frames: ColumnWindows,
	plan: ScenarioPlan,
	beat: number,
): number {
	const last = plan.beats.length - 1;
	let center = Number.NaN;
	for (let i = 0; i <= Math.min(beat, last - 1); i++) {
		center = pickWindow(frames, plan.beats[i].focus, center);
	}
	return beat >= last ? restWindow(frames, plan, center) : center;
}
