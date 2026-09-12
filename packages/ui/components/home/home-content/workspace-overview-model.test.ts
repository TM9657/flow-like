import { describe, expect, it } from "bun:test";
import type {
	IExecutionActivity,
	IExecutionUsageRecord,
} from "../../../lib/schema/usage/tracking";
import {
	workspaceProfileAppCount,
	workspacePulseHistory,
	workspacePulseMetrics,
	workspacePulseState,
} from "./workspace-overview-model";

const record = (id: string, status: string): IExecutionUsageRecord => ({
	id,
	status,
	created_at: "2026-09-11T10:00:00Z",
	app_id: "app",
	board_id: "board",
	node_id: "node",
	version: "1",
	instance: null,
	technical_user_id: null,
	microseconds: 100,
});

const activity = (
	overrides: Partial<IExecutionActivity> = {},
): IExecutionActivity => ({
	days: 7,
	from: "2026-09-05T00:00:00Z",
	to: "2026-09-11T14:00:00Z",
	buckets: [{ day: "2026-09-11", count: 4_861, attention_count: 15 }],
	apps: [{ app_id: "app", count: 4_861, attention_count: 15 }],
	total: 4_861,
	attention_total: 15,
	average_microseconds: 900,
	attention: [record("flagged", "Error")],
	...overrides,
});

const empty = () =>
	activity({
		buckets: [],
		apps: [],
		total: 0,
		attention_total: 0,
		average_microseconds: null,
		attention: [],
	});

describe("workspace pulse source truthfulness", () => {
	it("counts only accessible apps saved in the current profile and leaves missing sources unknown", () => {
		expect(
			workspaceProfileAppCount(["a", "b", "b", "hidden"], ["b", "deleted"]),
		).toBe(1);
		expect(workspaceProfileAppCount(["a", "b"], [])).toBe(0);
		expect(workspaceProfileAppCount(undefined, ["a"])).toBeUndefined();
		expect(workspaceProfileAppCount(["a"], undefined)).toBeUndefined();
	});

	it("keeps missing history distinct from a confirmed empty period", () => {
		expect(workspacePulseHistory(undefined)).toBeNull();
		const none = workspacePulseHistory(empty());
		expect(none?.total).toBe(0);
		expect(none?.attentionTotal).toBe(0);
	});

	it("reports the period total rather than a page size", () => {
		const counted = workspacePulseHistory(activity());
		expect(counted?.total).toBe(4_861);
		expect(counted?.attentionTotal).toBe(15);
		// The flagged list is capped for display; the count is not.
		expect(counted?.attention).toHaveLength(1);
		expect(counted?.attentionCapped).toBe(true);
	});

	it("keeps confirmed zero metrics while withholding disabled or failed cached sources", () => {
		const none = workspacePulseHistory(empty());
		expect(workspacePulseMetrics(none, true, false)?.total).toBe(0);
		expect(workspacePulseMetrics(none, false, false)).toBeNull();
		expect(workspacePulseMetrics(none, true, true)).toBeNull();
		expect(workspacePulseMetrics(null, true, false)).toBeNull();

		const cached = workspacePulseHistory(activity());
		expect(workspacePulseMetrics(cached, false, false)).toBeNull();
		expect(workspacePulseMetrics(cached, true, true)).toBeNull();
		expect(workspacePulseMetrics(cached, true, false)?.attention).toHaveLength(
			1,
		);
	});

	it("separates a starter workspace from a loading, failed or active one", () => {
		const base = {
			authenticated: true,
			supported: true,
			loading: false,
			error: false,
		};
		expect(workspacePulseState({ ...base, volume: 4_861 })).toBe("activity");
		expect(workspacePulseState({ ...base, volume: 0 })).toBe("starter");
		expect(
			workspacePulseState({ ...base, loading: true, volume: undefined }),
		).toBe("loading");
		// A cached count keeps the chart up while a refetch is in flight.
		expect(workspacePulseState({ ...base, loading: true, volume: 12 })).toBe(
			"activity",
		);
		expect(
			workspacePulseState({ ...base, error: true, volume: undefined }),
		).toBe("unavailable");
		expect(
			workspacePulseState({ ...base, authenticated: false, volume: 12 }),
		).toBe("starter");
		expect(workspacePulseState({ ...base, supported: false, volume: 12 })).toBe(
			"starter",
		);
	});
});
