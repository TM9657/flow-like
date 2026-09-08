import { describe, expect, test } from "bun:test";
import { getEventStatus } from "./event-status";

describe("event overview status", () => {
	const trigger = { active: true, blocking: false, requiresSink: true };

	test("keeps a pending or failed sink check separate from a stopped sink", () => {
		expect(getEventStatus(trigger)).toBe("unknown");
		expect(getEventStatus({ ...trigger, sinkActive: false })).toBe("attention");
		expect(getEventStatus({ ...trigger, sinkActive: true })).toBe("live");
	});

	test("reports a paused event even while its sink status is unavailable", () => {
		expect(getEventStatus({ ...trigger, active: false })).toBe("paused");
		expect(
			getEventStatus({ ...trigger, active: false, sinkActive: true }),
		).toBe("paused");
	});

	test("does not require a sink check for an interactive event", () => {
		expect(getEventStatus({ ...trigger, requiresSink: false })).toBe("live");
	});

	test("preserves blocking setup issues independently of activation", () => {
		for (const active of [true, false]) {
			expect(
				getEventStatus({
					...trigger,
					active,
					blocking: true,
					sinkActive: true,
				}),
			).toBe("attention");
		}
	});
});
