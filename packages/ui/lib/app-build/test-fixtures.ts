import type { AppSpec } from "./contract";

export function exampleAppSpec(): AppSpec {
	return {
		schema_version: 1,
		name: "Issue intake",
		description: "Accept and persist one issue.",
		requirements: [
			{
				id: "capture_issue",
				description: "Validate and save a submitted issue.",
			},
		],
		resources: [
			{
				key: "issue_flow",
				kind: "board",
				depends_on: [],
				requirement_ids: ["capture_issue"],
				config: {
					name: "Issue flow",
					instruction: "Validate the payload and persist the issue.",
				},
			},
			{
				key: "submit_issue",
				kind: "event",
				depends_on: ["issue_flow"],
				requirement_ids: ["capture_issue"],
				config: {
					name: "Submit issue",
					event_type: "quick_action",
					board: "issue_flow",
					entry_node: "submit",
				},
			},
		],
		scenarios: [
			{
				schema: "flowpilot.app-behavior-scenario/v1",
				id: "submit_issue_works",
				description: "Submitting a valid issue starts and completes a run.",
				requirement_ids: ["capture_issue"],
				target: { resource_key: "submit_issue", kind: "event" },
				steps: [
					{
						id: "submit",
						tool: "call_app_event",
						arguments: { payload: { title: "Broken export" } },
					},
				],
				assertions: [
					{
						id: "run_completed",
						kind: "run_outcome",
						step_id: "submit",
						run_id_path: "/run_id",
					},
					{
						id: "issue_saved",
						kind: "state",
						step_id: "submit",
						path: "/saved/title",
						operator: "equals",
						expected: "Broken export",
					},
				],
			},
		],
	};
}
