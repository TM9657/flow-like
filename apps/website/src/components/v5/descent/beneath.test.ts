import { describe, expect, test } from "bun:test";
import { layoutBoard } from "../board/board-layout";
import {
	BOARD_STRING_KEYS,
	type BoardStrings,
	createBoard,
} from "../board/board-spec";
import { createDirector } from "./beneath-director";
import {
	columnWindows,
	followBeat,
	followWindow,
	pickWindow,
} from "./beneath-follow";
import {
	HOLD,
	type PlanMark,
	type ScenarioPlan,
	buildPlans,
	planCycle,
	planLocal,
} from "./beneath-plan";
import type { DescentScenarioEvent } from "./types";

const strings = Object.fromEntries(
	BOARD_STRING_KEYS.map((key) => [key, key.split(".").pop() ?? key]),
) as BoardStrings;
const board = createBoard(strings);
const measure = (text: string, font: string) =>
	Array.from(text).length * 0.55 * Number(/([\d.]+)px/.exec(font)?.[1] ?? 10);
const layout = layoutBoard(board, { measure, fontFamily: "Inter" });
const plans = buildPlans(board, layout);

const planById = (id: string): ScenarioPlan => {
	const found = plans.find((p) => p.id === id);
	if (!found) throw new Error(`no plan ${id}`);
	return found;
};

describe("plans", () => {
	test("one plan per scenario, in data order", () => {
		expect(plans.map((p) => p.id)).toEqual([
			"approval",
			"insight",
			"flowpilot",
		]);
	});

	test("hop steps light the route pills in order, the last one on landing", () => {
		for (const p of plans) {
			const hops = p.marks.filter(
				(m): m is Extract<PlanMark, { kind: "hop" }> => m.kind === "hop",
			);
			expect(hops.map((m) => m.step)).toEqual([0, 1, 2]);
			const last = hops[hops.length - 1];
			const lastArrival = Math.max(...p.arrivals.map((a) => a.at));
			expect(last.at).toBeCloseTo(lastArrival, 6);
			expect(p.duration).toBeGreaterThan(last.at);
			expect(p.period).toBeCloseTo(p.duration + HOLD, 6);
			for (let i = 1; i < p.marks.length; i++) {
				expect(p.marks[i].at).toBeGreaterThanOrEqual(p.marks[i - 1].at);
			}
		}
	});

	test("arrivals reach the right windows", () => {
		const apps = (id: string) => planById(id).arrivals.map((a) => a.app);
		expect(apps("approval")).toEqual([1]);
		expect(apps("insight")).toEqual([0]);
		expect(apps("flowpilot").sort()).toEqual([0, 6, 7]);
	});

	test("Field Service gets the review once the agenda reaches Calendar", () => {
		const p = planById("flowpilot");
		const agenda = p.hops.find((h) => h.id === "pilot-calendar-agenda");
		const field = p.arrivals.find((a) => a.app === 7);
		expect(agenda && field).toBeTruthy();
		if (!agenda || !field) return;
		expect(field.at).toBeCloseTo(agenda.start + agenda.duration, 6);
	});

	test("each scenario ends in a thread climbing into its window", () => {
		expect(plans.map((p) => p.landing)).toEqual([
			"finance",
			"ops-ontology",
			"pilot",
		]);
	});

	test("a beat starts before the previous pulses land, so work hands over", () => {
		for (const p of plans) {
			for (let i = 1; i < p.beats.length; i++) {
				expect(p.beats[i].start).toBeLessThan(p.beats[i - 1].end);
				expect(p.beats[i].start).toBeGreaterThan(p.beats[i - 1].start);
			}
		}
	});

	test("the end state is held longer than the section's autoplay pause", () => {
		expect(HOLD).toBeGreaterThan(3.5);
	});
});

function recorder() {
	const events: DescentScenarioEvent[] = [];
	const log: string[] = [];
	const d = createDirector({
		emit: (e) => events.push(e),
		reset: (apps) => log.push(`reset ${[...apps].sort().join(",")}`),
		depart: (app) => log.push(`live ${app}`),
		arrive: (app) => log.push(`arrived ${app}`),
	});
	return { d, events, log };
}

const brief = (events: readonly DescentScenarioEvent[]) =>
	events.map((e) =>
		e.type === "hop"
			? `hop${e.step}`
			: e.type === "arrive"
				? `arrive${e.app}`
				: e.type,
	);

describe("director", () => {
	const approval = planById("approval");

	test("a run emits start, the hops, the arrival and complete, in order", () => {
		const { d, events } = recorder();
		for (let t = 0; t <= approval.duration + 0.05; t += 1 / 60) {
			d.play(approval, { index: 0, elapsed: t });
		}
		expect(brief(events)).toEqual([
			"start",
			"hop0",
			"hop1",
			"hop2",
			"arrive1",
			"complete",
		]);
		const first = events[0];
		expect(first.type === "start" && first.duration).toBeCloseTo(
			approval.duration,
			6,
		);
	});

	test("a frozen clock jumping forward replays what it skipped, once", () => {
		const { d, events } = recorder();
		d.play(approval, { index: 0, elapsed: 1.4 });
		d.play(approval, { index: 0, elapsed: 1.4 });
		expect(brief(events)).toEqual(["start", "hop0", "hop1"]);
		d.play(approval, { index: 0, elapsed: 4.6 });
		expect(brief(events)).toEqual(["start", "hop0", "hop1", "hop2", "arrive1"]);
	});

	test("after the hold the scenario replays on its own and resets its own windows", () => {
		const { d, events, log } = recorder();
		for (let t = 0; t <= approval.period + 0.2; t += 0.05) {
			d.play(approval, { index: 0, elapsed: t });
		}
		expect(brief(events).filter((e) => e === "start").length).toBe(2);
		expect(log.filter((l) => l.startsWith("reset")).length).toBe(2);
		expect(log).toContain("arrived 1");
	});

	test("a run resets only the windows it sends work from or delivers it to", () => {
		const { d, log } = recorder();
		d.play(planById("insight"), { index: 1, elapsed: 0.01 });
		expect(log[0]).toBe("reset 0,3");
	});

	test("replayScenario (the clock going back) and another scenario restart the run", () => {
		const { d, events } = recorder();
		d.play(approval, { index: 0, elapsed: 2 });
		d.play(approval, { index: 0, elapsed: 0.01 });
		d.play(plans[1], { index: 1, elapsed: 0.01 });
		expect(brief(events).filter((e) => e === "start").length).toBe(3);
		expect(events[events.length - 1]).toEqual({
			type: "start",
			scenario: 1,
			duration: plans[1].duration,
		});
	});

	test("FlowPilot Workspace goes live when it sends work and shows its end state when the briefing lands", () => {
		const { d, log } = recorder();
		const flowpilot = planById("flowpilot");
		d.play(flowpilot, { index: 2, elapsed: flowpilot.duration });
		expect(log).toEqual([
			"reset 0,6,7",
			"live 6",
			"arrived 7",
			"arrived 0",
			"arrived 6",
		]);
	});
});

describe("follow", () => {
	const frames = columnWindows(layout, 2 * layout.nodes[0].width + 60 + 24);

	test("a two-column frame sits on whole columns", () => {
		expect(frames.span).toBe(2 * layout.nodes[0].width + 60);
		const columns = [...new Set(layout.nodes.map((n) => n.x))].sort(
			(a, b) => a - b,
		);
		expect(frames.centers).toEqual(
			columns.slice(0, columns.length - 1).map((x) => x + frames.span / 2),
		);
	});

	test("the frame holds the landing point and stays put while it can", () => {
		const [first, second] = frames.centers;
		const shared = (first + second) / 2;
		expect(pickWindow(frames, shared, first)).toBe(first);
		expect(pickWindow(frames, shared, second)).toBe(second);
		const far = frames.centers[frames.centers.length - 1];
		expect(pickWindow(frames, far + 40, first)).toBe(far);
	});

	test("the still frames the arrival; a replay pre-rolls to the first beat", () => {
		const p = plans[0];
		expect(followBeat(p, 0, false)).toBe(p.beats.length - 1);
		expect(followBeat(p, p.period - 0.1, true)).toBe(0);
		expect(followBeat(p, p.beats[2].start, true)).toBe(2);
	});

	test("the frame for a beat follows the run from its start, so it never depends on the previous frame", () => {
		const flowpilot = planById("flowpilot");
		const last = flowpilot.beats.length - 1;
		const chained = followWindow(frames, flowpilot, last);
		const atCalendar = followWindow(frames, flowpilot, last - 1);
		expect(chained).toBe(atCalendar);
		const approval = planById("approval");
		const centers = approval.beats.map((_, i) =>
			followWindow(frames, approval, i),
		);
		expect(centers).toEqual([...centers].sort((a, b) => a - b));
		expect(centers[0]).toBe(frames.centers[0]);
		expect(centers[centers.length - 1]).toBe(
			frames.centers[frames.centers.length - 1],
		);
	});
});

describe("plan data for the windows and threads", () => {
	test("delivered apps are the arrivals; departures that also receive work count once", () => {
		expect([...planById("approval").delivered].sort()).toEqual([1]);
		expect([...planById("insight").delivered].sort()).toEqual([0]);
		expect([...planById("flowpilot").delivered].sort((a, b) => a - b)).toEqual([
			0, 6, 7,
		]);
		expect(planById("insight").apps).toContain(3);
		expect(planById("insight").delivered).not.toContain(3);
	});

	test("each plan knows the threads its pulses run along", () => {
		for (const p of plans) {
			const ids = p.hops.filter((h) => h.kind === "thread").map((h) => h.id);
			expect([...p.threads].sort()).toEqual([...new Set(ids)].sort());
		}
	});

	test("each story draws one to three threads, and every one lands on its own route", () => {
		for (const p of plans) {
			expect(p.threads.size).toBeGreaterThanOrEqual(1);
			expect(p.threads.size).toBeLessThanOrEqual(3);
			const route = new Set(layout.routes[p.id].nodes);
			for (const id of p.threads) {
				const thread = board.threads.find((entry) => entry.id === id);
				expect(thread && route.has(thread.node)).toBe(true);
			}
		}
	});

	test("planCycle and planLocal split the clock into runs", () => {
		const p = plans[0];
		expect(planCycle(p, 0)).toBe(0);
		expect(planLocal(p, 0.5)).toBeCloseTo(0.5, 9);
		expect(planCycle(p, p.period * 2 + 0.25)).toBe(2);
		expect(planLocal(p, p.period * 2 + 0.25)).toBeCloseTo(0.25, 9);
		expect(planLocal(p, -3)).toBe(0);
	});
});
