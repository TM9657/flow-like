import type { IBackendState } from "@flow-like/flow-like-ui/state/backend-state";
import { describe, expect, test, vi } from "vitest";
import { provisionIntakeReliabilityApp } from "../intake-provisioning";

function fixture(nonempty = false) {
	let app: Record<string, unknown> = {
		id: "fresh",
		status: "Active",
		visibility: "Private",
		boards: ["default-board"],
	};
	const board = {
		id: "default-board",
		name: "Main",
		nodes: nonempty ? { node: {} } : {},
		layers: {},
		variables: {},
		comments: {},
		refs: {},
		page_ids: [],
	};
	let fields: Record<string, unknown>[] | undefined;
	const backend = {
		appState: {
			createApp: vi.fn(async () => app),
			getAppAuthoritative: async () => app,
			updateAppAuthoritative: async (next: Record<string, unknown>) => {
				app = next;
			},
		},
		userState: {
			getSettingsProfile: async () => ({ id: "local" }),
			updateProfileApp: vi.fn(async () => undefined),
		},
		boardState: {
			getBoardSummariesAuthoritative: async () => [
				{ id: board.id, name: board.name },
			],
			getBoardAuthoritative: async () => board,
			upsertBoard: vi.fn(
				async (_appId: string, boardId: string, name: string) => {
					if (boardId !== board.id) throw new Error("Unexpected extra board");
					board.name = name;
				},
			),
			getFlowScriptAuthoritative: async () => "",
		},
		dbState: {
			listTablesAuthoritative: async () => (fields ? ["intake_tickets"] : []),
			createTable: vi.fn(
				async (
					_app: string,
					_name: string,
					columns: Record<string, unknown>[],
				) => {
					fields = columns.map((column) => ({
						name: column.name,
						data_type: column.type === "string" ? "Utf8" : "Int64",
						nullable: column.nullable,
					}));
					return { created: true };
				},
			),
			getSchemaAuthoritative: async () => ({ fields }),
		},
		pageState: { getPagesAuthoritative: async () => [] },
		widgetState: { getWidgetsAuthoritative: async () => [] },
		eventState: { getEventsAuthoritative: async () => [] },
	} as unknown as IBackendState;
	return { backend, board };
}

describe("fixed intake resource provisioning", () => {
	test("reuses the fresh default board and returns exact typed resource evidence before generation", async () => {
		const f = fixture();
		const setup = await provisionIntakeReliabilityApp(
			f.backend,
			"Reliability run",
		);
		expect(setup.boardId).toBe("default-board");
		expect(f.board.name).toBe("Intake Workflow");
		expect(f.backend.boardState.upsertBoard).toHaveBeenCalledTimes(1);
		expect(setup.setupEvidence.tables[0]).toMatchObject({
			name: "intake_tickets",
			fields: [
				{ name: "summary", data_type: "Utf8", nullable: false },
				{ name: "queue", data_type: "Utf8", nullable: false },
				{ name: "response_minutes", data_type: "Int64", nullable: false },
			],
		});
		expect(
			new Set([setup.pageId, setup.submitEventId, setup.pageEventId]).size,
		).toBe(3);
		expect(setup.promptContext).toContain(setup.submitEventId);
		expect(setup.promptContext).toContain("summary_input");
		expect(setup.promptContext).toContain(
			"First delegate the complete workflow",
		);
		expect(setup.promptContext).toContain(
			"Then delegate creation of the reserved page",
		);
		expect(setup.promptContext.indexOf("First delegate")).toBeLessThan(
			setup.promptContext.indexOf("Then delegate"),
		);
		expect(setup.promptContext).toContain(
			"eventsGeneric submitTicket(payload: Struct)",
		);
		expect(setup.promptContext).not.toContain("accepting summary:string");
		expect(setup.promptContext).toContain(
			`ui::getElementValue({ elementRef: ui::getElement({ elementRef: "${setup.pageId}/summary_input" }).element }).value`,
		);
		expect(setup.promptContext).toContain(
			`ui::setElementText({ elementRef: "${setup.pageId}/queue_result", text: queue })`,
		);
		expect(setup.promptContext).toContain("submit_ticket.eventHandlers.click");
		expect(setup.promptContext).toContain("nodeId:<returned entry node ID>");
		expect(setup.promptContext).toContain(
			"Leave pageAction metadata to the native Page bootstrap",
		);
		expect(setup.promptContext).toContain(
			"no Data Studio delegation is needed",
		);
		expect(
			(await f.backend.appState.getAppAuthoritative(setup.appId)).status,
		).toBe("Inactive");
	});
	test("does not reuse a default board containing workflow content", async () => {
		const f = fixture(true);
		await expect(
			provisionIntakeReliabilityApp(f.backend, "Reliability run"),
		).rejects.toThrow("already contains resources");
		expect(f.backend.dbState.createTable).not.toHaveBeenCalled();
		expect(f.backend.boardState.upsertBoard).not.toHaveBeenCalled();
	});
});
