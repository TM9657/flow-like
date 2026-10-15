import { describe, expect, test } from "bun:test";
import { nodeExecutionState } from "./execution-state";

describe("node execution controls", () => {
	test("a repeated live invocation supplies the Stop target instead of an older completed invocation", () => {
		const runs = new Map([
			[
				"older",
				{
					nodes: new Set(["downstream"]),
					already_executed: new Set(["event"]),
				},
			],
			[
				"live",
				{ nodes: new Set(["event"]), already_executed: new Set<string>() },
			],
		]);
		expect(nodeExecutionState(runs, "event")).toEqual({
			status: "running",
			runId: "live",
		});
		// Finishing the event still leaves a cancellable workflow while its children run.
		runs.get("live")?.nodes.delete("event");
		runs.get("live")?.already_executed.add("event");
		expect(nodeExecutionState(runs, "event")).toEqual({
			status: "done",
			runId: "older",
		});
		runs.delete("older");
		expect(nodeExecutionState(runs, "event")).toEqual({
			status: "done",
			runId: "live",
		});
		runs.delete("live");
		expect(nodeExecutionState(runs, "event")).toEqual({ status: "none" });
	});

	test("unrelated running nodes do not expose a Stop action for an idle event", () => {
		const runs = new Map([
			[
				"other",
				{
					nodes: new Set(["other-node"]),
					already_executed: new Set(["another-node"]),
				},
			],
		]);
		expect(nodeExecutionState(runs, "event")).toEqual({ status: "none" });
	});
});
