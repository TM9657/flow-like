import { reserveAppResourceId } from "@flow-like/flow-like-ui/lib/app-build/fingerprint";
import { provisionAppBuildEvents } from "@flow-like/flow-like-ui/lib/app-build/resource-events";
import {
	type AppBuildFoundationEvidence,
	provisionAppBuildFoundation,
} from "@flow-like/flow-like-ui/lib/app-build/resource-foundation";
import { beginAppBuildStaging } from "@flow-like/flow-like-ui/lib/app-build/staging";
import {
	IExecutionStage,
	ILogLevel,
} from "@flow-like/flow-like-ui/lib/schema/flow/board";
import { nowSystemTime } from "@flow-like/flow-like-ui/lib/time/now";
import type { IBackendState } from "@flow-like/flow-like-ui/state/backend-state";

export const INTAKE_RELIABILITY_CONTRACT = {
	buildId: "intake-reliability-v1",
	boardName: "Intake Workflow",
	pageName: "intake_console",
	route: "/intake",
	tableName: "intake_tickets",
	entryName: "submitTicket",
	componentIds: {
		summary: "summary_input",
		submit: "submit_ticket",
		result: "queue_result",
	},
	summaryPath: "/summary",
	columns: [
		{ name: "summary", type: "string" as const, nullable: false },
		{ name: "queue", type: "string" as const, nullable: false },
		{ name: "response_minutes", type: "int64" as const, nullable: false },
	],
};

export interface IntakeReliabilitySetup {
	readonly appId: string;
	readonly boardId: string;
	readonly pageId: string;
	readonly submitEventId: string;
	readonly pageEventId: string;
	readonly tableName: string;
	readonly setupEvidence: AppBuildFoundationEvidence;
	readonly promptContext: string;
}

/** The fixed reliability fixture uses the same host setup operations as AppSpec builds. */
export async function provisionIntakeReliabilityApp(
	backend: IBackendState,
	expectedAppName: string,
	assertActive: () => void = () => undefined,
): Promise<IntakeReliabilitySetup> {
	assertActive();
	const time = nowSystemTime();
	const app = await backend.appState.createApp(
		{
			name: expectedAppName,
			description: "Isolated support intake reliability fixture.",
			tags: [],
			use_case: "",
			created_at: time,
			updated_at: time,
			preview_media: [],
		},
		[],
		false,
	);
	assertActive();
	const profile = await backend.userState.getSettingsProfile();
	await backend.userState.updateProfileApp(
		profile,
		{ app_id: app.id, favorite: false, pinned: false },
		"Upsert",
	);
	await beginAppBuildStaging(backend, app.id, assertActive);
	const boards = await backend.boardState.getBoardSummariesAuthoritative(
		app.id,
	);
	if (boards.length > 1)
		throw new Error("Fresh intake app has multiple default boards.");
	const contract = INTAKE_RELIABILITY_CONTRACT;
	const reserve = (kind: "board" | "page" | "event", key: string) =>
		reserveAppResourceId(app.id, contract.buildId, kind, key);
	const boardId = boards[0]?.id ?? reserve("board", "intake_flow");
	if (boards[0]) {
		const board = await backend.boardState.getBoardAuthoritative(
			app.id,
			boardId,
		);
		if (
			Object.keys(board.nodes ?? {}).length ||
			Object.keys(board.layers ?? {}).length ||
			Object.keys(board.variables ?? {}).length ||
			(board.page_ids ?? []).length
		)
			throw new Error(
				"Fresh intake default board contains resources and cannot be reused.",
			);
		assertActive();
		await backend.boardState.upsertBoard(
			app.id,
			boardId,
			contract.boardName,
			"Support intake workflow",
			ILogLevel.Info,
			IExecutionStage.Dev,
		);
	}
	const setupEvidence = await provisionAppBuildFoundation(
		backend,
		{
			app_id: app.id,
			boards: [{ id: boardId, name: contract.boardName }],
			tables: [{ name: contract.tableName, columns: contract.columns }],
		},
		{ assertActive },
	);
	const pageId = reserve("page", "intake_console");
	const submitEventId = reserve("event", "submit_ticket");
	const pageEventId = reserve("event", "intake_page");
	return {
		appId: app.id,
		boardId,
		pageId,
		submitEventId,
		pageEventId,
		tableName: contract.tableName,
		setupEvidence,
		promptContext: [
			"The host has already created the destination app, its workflow board and the exact typed table. Complete this existing app using these identifiers verbatim.",
			JSON.stringify({
				app_id: app.id,
				board_id: boardId,
				board_name: contract.boardName,
				page_id: pageId,
				page_name: contract.pageName,
				route: contract.route,
				table_name: contract.tableName,
				columns: contract.columns,
				entry_name: contract.entryName,
				submit_event_id: submitEventId,
				page_event_id: pageEventId,
				component_ids: contract.componentIds,
				summary_data_path: contract.summaryPath,
				summary_element_ref: `${pageId}/${contract.componentIds.summary}`,
				queue_element_ref: `${pageId}/${contract.componentIds.result}`,
			}),
			"First delegate the complete workflow to the existing board. Use eventsGeneric submitTicket(payload: Struct), implement the requested routeRecord policy and persistence, then commit and read back its exact entry node ID. The table is already typed and ready; no Data Studio delegation is needed. The reserved page does not exist yet. Use its known selectors in the workflow without reading or creating a placeholder page.",
			`Read the live summary with ui::getElementValue({ elementRef: ui::getElement({ elementRef: ${JSON.stringify(`${pageId}/${contract.componentIds.summary}`)} }).element }).value. After inserting the ticket, display its queue with ui::setElementText({ elementRef: ${JSON.stringify(`${pageId}/${contract.componentIds.result}`)}, text: queue }). These values come from the Page runtime's element payload; context.summary does not bind a top-level summary event pin. Confirm the live catalog declarations for these UI calls when drafting.`,
			`Then delegate creation of the reserved page on that same board, passing the exact persisted submitTicket node ID. Set summary_input.value to {path:"/summary"}. Set submit_ticket.eventHandlers.click to exactly one {name:"workflow_event",context:{nodeId:<returned entry node ID>}} action. Use the returned workflow node ID, not the reserved app Event ID or the name submitTicket. Leave pageAction metadata to the native Page bootstrap. Keep load, unload and interval handlers unset. Preserve the exact component IDs and the provisioned table schema.`,
			'Read back the completed page with flowpilot_widget mode="inspect" and the exact app_id, board_id, and page_id. This returns persisted component, action, and lifecycle facts without a new UI generation. Read back the workflow with flowpilot_board mode="inspect" and the exact app_id and board_id. Use its stored source and entry IDs as facts; a specialist explanation is not the persisted source. The host registers the reserved inactive app Events after generation, so leave Event registration and activation to the host.',
		].join("\n\n"),
	};
}

export async function finalizeIntakeReliabilityEvents(
	backend: IBackendState,
	setup: IntakeReliabilitySetup,
	assertActive: () => void = () => undefined,
) {
	return provisionAppBuildEvents(
		backend,
		setup.appId,
		[
			{
				id: setup.submitEventId,
				name: "Submit ticket",
				event_type: "generic_form",
				board_id: setup.boardId,
				entry_node: INTAKE_RELIABILITY_CONTRACT.entryName,
			},
			{
				id: setup.pageEventId,
				name: "Intake console",
				event_type: "page",
				board_id: setup.boardId,
				page_id: setup.pageId,
				route: INTAKE_RELIABILITY_CONTRACT.route,
			},
		],
		{ assertActive },
	);
}
