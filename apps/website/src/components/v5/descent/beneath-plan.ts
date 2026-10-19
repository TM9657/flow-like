import type { BoardLayout } from "../board/board-layout";
import { PULSE } from "../board/board-pulse";
import type { BoardScenarioId, BoardSpec } from "../board/board-spec";

/** Seconds a pulse needs along a board wire: egui's sweep. */
const EDGE_RUN = PULSE.durationMs / 1000;
/** Threads are longer than wires, so their pulse runs a little longer. */
const THREAD_RUN = 1.4;
/** The next beat leaves once the previous pulses are this far along. */
const HANDOFF = 0.85;
/** Pause between the last landing and "complete". */
const SETTLE = 0.3;
/**
 * End state held after "complete" before the scenario replays: longer than the
 * section's autoplay pause (controller AUTOPLAY_DELAY), so autoplay moves on
 * before a replay and every end state stays up while the copy is read.
 */
export const HOLD = 3.8;
/** Step 0 lights as the first pulse leaves its source. */
const DEPART_DELAY = 0.05;

export interface PlanHop {
	readonly beat: number;
	readonly kind: "edge" | "thread";
	readonly id: string;
	readonly toApp: boolean;
	readonly start: number;
	readonly duration: number;
}

export type PlanMark =
	| { readonly at: number; readonly kind: "hop"; readonly step: number }
	| { readonly at: number; readonly kind: "arrive"; readonly app: number }
	| { readonly at: number; readonly kind: "depart"; readonly app: number };

export interface PlanBeat {
	readonly start: number;
	readonly end: number;
	/** Board x where the beat's pulses land, on average: what a narrow frame follows. */
	readonly focus: number;
}

export interface ScenarioPlan {
	readonly id: BoardScenarioId;
	readonly hops: readonly PlanHop[];
	/** Timed marks, in order. */
	readonly marks: readonly PlanMark[];
	readonly beats: readonly PlanBeat[];
	/** Seconds from "start" to "complete". */
	readonly duration: number;
	/** One run plus the held end state. */
	readonly period: number;
	/** Apps the scenario sends work from or delivers it to. */
	readonly apps: readonly number[];
	/** Apps the scenario delivers work to. */
	readonly delivered: readonly number[];
	readonly arrivals: readonly { readonly app: number; readonly at: number }[];
	readonly departures: readonly { readonly app: number; readonly at: number }[];
	/** The thread the last pulse climbs into its window, if the scenario ends in one. */
	readonly landing: string | null;
	/** Every thread the scenario's pulses run along. */
	readonly threads: ReadonlySet<string>;
}

/**
 * Apps whose end state follows a wire rather than a thread: Calendar has no
 * thread of its own, so Field Service shows the scheduled review once the
 * agenda reaches Calendar.
 */
const WIRE_ARRIVALS: Partial<
	Record<
		BoardScenarioId,
		readonly { readonly app: number; readonly edge: string }[]
	>
> = {
	flowpilot: [{ app: 7, edge: "pilot-calendar-agenda" }],
};

/** Board x where a hop's pulse lands: the target pin, or the thread's reference square. */
function landingX(layout: BoardLayout, hop: PlanHop): number {
	if (hop.kind === "thread") return layout.threadAnchor(hop.id)[0];
	const path = layout.edgePath(hop.id);
	return path[path.length - 1][0];
}

function scheduleHops(spec: BoardSpec, id: BoardScenarioId) {
	const scenario = spec.scenarios.find((entry) => entry.id === id);
	if (!scenario) throw new Error(`Descent: no board scenario "${id}"`);
	const hops: PlanHop[] = [];
	const stages: number[] = [];
	let start = 0;
	for (const [beat, entry] of scenario.beats.entries()) {
		const edges = entry.edges.map(
			(edge): PlanHop => ({
				beat,
				kind: "edge",
				id: edge,
				toApp: false,
				start,
				duration: EDGE_RUN,
			}),
		);
		const threads = entry.threads.map(
			(thread): PlanHop => ({
				beat,
				kind: "thread",
				id: thread.id,
				toApp: thread.toApp,
				start,
				duration: THREAD_RUN,
			}),
		);
		hops.push(...edges, ...threads);
		stages.push(entry.stage);
		start +=
			HANDOFF * Math.max(...[...edges, ...threads].map((hop) => hop.duration));
	}
	return { hops, stages };
}

function appOf(spec: BoardSpec, threadId: string): number {
	const thread = spec.threads.find((entry) => entry.id === threadId);
	if (!thread) throw new Error(`Descent: no board thread "${threadId}"`);
	return thread.app;
}

const landing = (hop: PlanHop) => hop.start + hop.duration;

function scheduleMarks(
	spec: BoardSpec,
	id: BoardScenarioId,
	hops: readonly PlanHop[],
	stages: readonly number[],
) {
	const lastStage = Math.max(...stages);
	const arrivals = hops
		.filter((hop) => hop.kind === "thread" && hop.toApp)
		.map((hop) => ({ app: appOf(spec, hop.id), at: landing(hop) }));
	for (const extra of WIRE_ARRIVALS[id] ?? []) {
		const hop = hops.find(
			(entry) => entry.kind === "edge" && entry.id === extra.edge,
		);
		if (hop) arrivals.push({ app: extra.app, at: landing(hop) });
	}
	const departures = hops
		.filter((hop) => hop.kind === "thread" && !hop.toApp)
		.map((hop) => ({ app: appOf(spec, hop.id), at: hop.start }));
	const finalHops = hops.filter((hop) => stages[hop.beat] === lastStage);
	const arrived = Math.max(...finalHops.map(landing));
	const marks: PlanMark[] = [];
	for (let step = 0; step < lastStage; step++) {
		const first = hops.find((hop) => stages[hop.beat] === step);
		if (first)
			marks.push({
				at: first.start + (step === 0 ? DEPART_DELAY : 0),
				kind: "hop",
				step,
			});
	}
	marks.push({ at: arrived, kind: "hop", step: lastStage });
	for (const { app, at } of departures) marks.push({ at, kind: "depart", app });
	for (const { app, at } of arrivals) marks.push({ at, kind: "arrive", app });
	const order = { depart: 0, hop: 1, arrive: 2 } as const;
	marks.sort((a, b) => a.at - b.at || order[a.kind] - order[b.kind]);
	return { marks, arrivals, departures, arrived };
}

function buildPlan(
	spec: BoardSpec,
	layout: BoardLayout,
	id: BoardScenarioId,
): ScenarioPlan {
	const { hops, stages } = scheduleHops(spec, id);
	const { marks, arrivals, departures, arrived } = scheduleMarks(
		spec,
		id,
		hops,
		stages,
	);
	const beats = stages.map((_, beat): PlanBeat => {
		const members = hops.filter((hop) => hop.beat === beat);
		const xs = members.map((hop) => landingX(layout, hop));
		return {
			start: Math.min(...members.map((hop) => hop.start)),
			end: Math.max(...members.map(landing)),
			focus: xs.reduce((sum, x) => sum + x, 0) / xs.length,
		};
	});
	const duration = arrived + SETTLE;
	const climbs = hops.filter((hop) => hop.kind === "thread" && hop.toApp);
	return {
		id,
		hops,
		marks,
		beats,
		duration,
		period: duration + HOLD,
		apps: [...new Set([...departures, ...arrivals].map((entry) => entry.app))],
		delivered: [...new Set(arrivals.map((entry) => entry.app))],
		arrivals,
		departures,
		landing: climbs.length > 0 ? climbs[climbs.length - 1].id : null,
		threads: new Set(
			hops.filter((hop) => hop.kind === "thread").map((hop) => hop.id),
		),
	};
}

/** One plan per scenario, in DescentData.scenarios order. */
export function buildPlans(
	spec: BoardSpec,
	layout: BoardLayout,
): ScenarioPlan[] {
	return spec.scenarios.map((scenario) => buildPlan(spec, layout, scenario.id));
}

/** The run a scenario is in, `elapsed` seconds after it started. */
export const planCycle = (plan: ScenarioPlan, elapsed: number) =>
	Math.floor(Math.max(0, elapsed) / plan.period);

/** Seconds into the run a scenario is in, `elapsed` seconds after it started. */
export const planLocal = (plan: ScenarioPlan, elapsed: number) =>
	Math.max(0, elapsed) - planCycle(plan, elapsed) * plan.period;
