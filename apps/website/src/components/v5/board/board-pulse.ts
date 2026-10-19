import { type RGBA, mix } from "./board-paint";

/** egui's activation sweep. Widths are screen px, the way egui sizes them regardless of zoom. */
export const PULSE = {
	durationMs: 1200,
	trailAlpha: 0.65,
	glowExtraWidth: 5,
	glowAlpha: 0.1,
	headAlpha: 0.85,
	headRadius: 1.1,
	primaryMix: 0.6,
} as const;

export type PulsePoint = readonly number[];

export interface PulseFrame {
	/** Fade of the whole comet over its run, sin(π·progress). */
	readonly opacity: number;
	readonly head: PulsePoint;
	/** Up to three path samples behind the head, then the head itself. */
	readonly trail: readonly PulsePoint[];
}

/** A frame `pulseAt` writes into, so a comet redrawn every frame allocates nothing. */
export interface PulseFrameTarget {
	opacity: number;
	head: number[];
	trail: PulsePoint[];
}

export const createPulseFrame = (): PulseFrameTarget => ({
	opacity: 0,
	head: [],
	trail: [],
});

/**
 * The comet at `progress` (0..1) along a sampled path, such as `edgePath()` (board units)
 * or a thread's 3D samples; written into `out` when given. Null once the run is over.
 */
export function pulseAt(
	path: readonly PulsePoint[],
	progress: number,
	out: PulseFrameTarget = createPulseFrame(),
): PulseFrame | null {
	if (!(progress >= 0 && progress < 1) || path.length < 2) return null;
	const position = progress * (path.length - 1);
	const first = Math.min(Math.floor(position), path.length - 2);
	const fraction = position - first;
	const a = path[first];
	const b = path[first + 1];
	out.head.length = a.length;
	for (let axis = 0; axis < a.length; axis++) {
		out.head[axis] = a[axis] + (b[axis] - a[axis]) * fraction;
	}
	out.trail.length = 0;
	for (let i = Math.max(0, first - 2); i <= first; i++) out.trail.push(path[i]);
	out.trail.push(out.head);
	out.opacity = Math.sin(Math.PI * progress);
	return out;
}

/** Trail width in screen px at a board zoom; the soft glow adds `PULSE.glowExtraWidth`. */
export const pulseWidth = (zoom: number) =>
	Math.min(2.5, Math.max(1, 2 * zoom));

/** The comet colour of a wire: its own colour pulled 60% toward primary. */
export const pulseTint = (wire: RGBA, primary: RGBA): RGBA =>
	mix(wire, primary, PULSE.primaryMix);
