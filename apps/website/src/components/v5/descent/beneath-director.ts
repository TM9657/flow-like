import {
	type PlanMark,
	type ScenarioPlan,
	planCycle,
	planLocal,
} from "./beneath-plan";
import type { ScenarioClock } from "./internal";
import type { DescentScenarioEvent } from "./types";

export interface DirectorOutput {
	emit(event: DescentScenarioEvent): void;
	/**
	 * A run starts over: the windows it sends work from or delivers it to go back
	 * to idle; every other window keeps the end state an earlier run left.
	 */
	reset(apps: readonly number[]): void;
	/** Work leaves app `app`: its badge turns live. */
	depart(app: number): void;
	/** Work reaches app `app`: its window shows its end state. */
	arrive(app: number): void;
}

export interface Director {
	/**
	 * Plays the scenario at `clock`: on a new run it resets the run's windows and emits
	 * "start", then every mark up to now in order, then "complete". A jump in the
	 * clock replays what it skipped, so a frozen clock yields the same events.
	 */
	play(plan: ScenarioPlan, clock: ScenarioClock): void;
	/** Stops tracking runs (reduced motion emits nothing); the next play starts over. */
	idle(): void;
}

export function createDirector(out: DirectorOutput): Director {
	const run = { index: -1, cycle: -1, elapsed: 0 };
	let next = 0;
	let completed = false;

	function fire(scenario: number, mark: PlanMark) {
		if (mark.kind === "hop") {
			out.emit({ type: "hop", scenario, step: mark.step });
		} else if (mark.kind === "depart") {
			out.depart(mark.app);
		} else {
			out.arrive(mark.app);
			out.emit({ type: "arrive", scenario, app: mark.app });
		}
	}

	return {
		play(plan, clock) {
			const cycle = planCycle(plan, clock.elapsed);
			const local = planLocal(plan, clock.elapsed);
			if (
				run.index !== clock.index ||
				run.cycle !== cycle ||
				clock.elapsed < run.elapsed
			) {
				out.reset(plan.apps);
				next = 0;
				completed = false;
				out.emit({
					type: "start",
					scenario: clock.index,
					duration: plan.duration,
				});
			}
			run.index = clock.index;
			run.cycle = cycle;
			run.elapsed = clock.elapsed;
			while (next < plan.marks.length && plan.marks[next].at <= local) {
				fire(clock.index, plan.marks[next++]);
			}
			if (!completed && local >= plan.duration) {
				completed = true;
				out.emit({ type: "complete", scenario: clock.index });
			}
		},
		idle() {
			run.index = -1;
		},
	};
}
