import { describe, expect, test } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import { compileAppSpec } from "./compiler";
import { resolveBuildEntry } from "./resource-links";
import { readAppBuildResource } from "./resource-readback";
import { exampleAppSpec } from "./test-fixtures";

function fixture() {
	const spec = exampleAppSpec();
	const plan = compileAppSpec(spec, { app_id: "app", build_id: "build" });
	const board = plan.resources.find((resource) => resource.kind === "board")!;
	const event = plan.resources.find((resource) => resource.kind === "event")!;
	const nodes = {
		submit: {
			id: "submit",
			name: "events_simple",
			friendly_name: "submit",
			pins: {
				out: {
					id: "out",
					pin_type: "Output",
					data_type: "Execution",
					connected_to: ["in"],
				},
			},
		},
		other: {
			id: "other",
			name: "events_simple",
			friendly_name: "Other action",
			pins: {
				other_out: {
					id: "other_out",
					pin_type: "Output",
					data_type: "Execution",
					connected_to: ["in"],
				},
			},
		},
		save: {
			id: "save",
			name: "save_record",
			pins: {
				in: {
					id: "in",
					pin_type: "Input",
					data_type: "Execution",
					connected_to: ["out"],
				},
			},
		},
	};
	const backend = {
		boardState: {
			getBoardAuthoritative: async () => ({ id: board.physical_id, nodes }),
		},
		eventState: {
			getEventsAuthoritative: async () => {
				return [
					{
						id: event.physical_id,
						name: "Submit",
						event_type: "quick_action",
						board_id: board.physical_id,
						node_id: "other",
					},
				];
			},
		},
	} as unknown as IBackendState;
	return { backend, plan, board, event, nodes };
}

describe("persisted app resource links", () => {
	test("resolves exactly one compatible runnable entry", async () => {
		const f = fixture();
		expect(
			await resolveBuildEntry(
				f.backend,
				"app",
				f.board.physical_id,
				"submit",
				"quick_action",
			),
		).toBe("submit");
		await expect(
			resolveBuildEntry(
				f.backend,
				"app",
				f.board.physical_id,
				"submit",
				"simple_chat",
			),
		).rejects.toThrow("0 compatible");
	});
	test("does not guess among ambiguous names or accept an unconnected entry", async () => {
		const f = fixture();
		f.nodes.other.friendly_name = "submit";
		await expect(
			resolveBuildEntry(f.backend, "app", f.board.physical_id, "submit"),
		).rejects.toThrow("2 compatible");
		f.nodes.submit.pins.out.connected_to = [];
		f.nodes.other.pins.other_out.connected_to = [];
		await expect(
			resolveBuildEntry(f.backend, "app", f.board.physical_id, "submit"),
		).rejects.toThrow("0 compatible");
	});
	test("readback rejects an Event linked to another valid entry in the same board", async () => {
		const f = fixture();
		expect(
			(await readAppBuildResource(f.backend, f.plan, f.event)).issues,
		).toContain("Event targets a different workflow entry.");
	});
});
