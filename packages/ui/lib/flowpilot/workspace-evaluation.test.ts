import { afterEach, describe, expect, it, vi } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import {
	dispatchWorkspaceEvaluationRead,
	getWorkspaceEvaluation,
	registerWorkspaceBuildEvaluation,
	registerWorkspaceEvaluation,
	registerWorkspaceEvaluationDestination,
	unregisterWorkspaceEvaluation,
	workspaceEvaluationAllowsApp,
	workspaceEvaluationMutationError,
	workspaceEvaluationTargetError,
} from "./workspace-evaluation";
import { rankWorkspaceDocuments as baseline } from "./workspace-ranker-baseline";
import {
	type WorkspaceDocument,
	workspaceResourceId,
	workspaceRevision,
} from "./workspace-resource";

afterEach(() => {
	unregisterWorkspaceEvaluation("conversation");
	vi.unstubAllEnvs();
});
describe("host controlled retrieval evaluation", () => {
	it("allows source board inspection while preserving mutation and scope restrictions", () => {
		vi.stubEnv("NODE_ENV", "development");
		const evaluation = registerWorkspaceEvaluation(
			"conversation",
			"improved",
			"source",
		);
		for (const mode of ["inspect", "explain"])
			expect(
				workspaceEvaluationTargetError(evaluation, "flowpilot_board", {
					app_id: "source",
					board_id: "board",
					mode,
				}),
			).toBeUndefined();
		for (const mode of ["edit", undefined])
			expect(
				workspaceEvaluationTargetError(evaluation, "flowpilot_board", {
					app_id: "source",
					mode,
				}),
			).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		expect(
			workspaceEvaluationTargetError(evaluation, "flowpilot_board", {
				app_id: "unregistered",
				board_id: "board",
				mode: "inspect",
			}),
		).toMatchObject({ code: "EVALUATION_APP_OUT_OF_SCOPE" });
	});

	it("allows source page inspection while preserving edit and destination restrictions", () => {
		vi.stubEnv("NODE_ENV", "development");
		const evaluation = registerWorkspaceEvaluation(
			"conversation",
			"improved",
			"source",
		);
		expect(
			workspaceEvaluationTargetError(evaluation, "flowpilot_widget", {
				app_id: "source",
				page_id: "page",
				mode: "inspect",
			}),
		).toBeUndefined();
		for (const mode of ["create", "edit"]) {
			expect(
				workspaceEvaluationTargetError(evaluation, "flowpilot_widget", {
					app_id: "source",
					mode,
				}),
			).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		}
		expect(
			workspaceEvaluationTargetError(evaluation, "flowpilot_widget", {
				app_id: "unregistered",
				mode: "inspect",
			}),
		).toMatchObject({ code: "EVALUATION_APP_OUT_OF_SCOPE" });
	});

	it("refuses registration outside a development runtime", () => {
		vi.stubEnv("NODE_ENV", "production");
		expect(() =>
			registerWorkspaceEvaluation("conversation", "baseline", "source"),
		).toThrow("development");
	});
	it("binds a mode to one conversation and protects the source from mutations", () => {
		vi.stubEnv("NODE_ENV", "development");
		const evaluation = registerWorkspaceEvaluation(
			"conversation",
			"baseline",
			"source",
		);
		expect(getWorkspaceEvaluation("other")).toBeUndefined();
		expect(getWorkspaceEvaluation("conversation")).toBe(evaluation);
		expect(() =>
			registerWorkspaceEvaluation("conversation", "improved", "source"),
		).toThrow("already");
		expect(
			workspaceEvaluationMutationError(evaluation, "upsert_event", {
				app_id: "source",
			}),
		).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		expect(
			workspaceEvaluationMutationError(evaluation, "build_flow", {
				appId: "target",
			}),
		).toBeUndefined();
		expect(
			workspaceEvaluationMutationError(evaluation, "build_flow", {}, "source"),
		).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		expect(
			workspaceEvaluationMutationError(evaluation, "read_flowscript_source", {
				app_id: "source",
			}),
		).toBeUndefined();
		unregisterWorkspaceEvaluation("conversation");
		expect(getWorkspaceEvaluation("conversation")).toBeUndefined();
	});
	it("resolves nonempty aliases and late targets before protecting the source", () => {
		vi.stubEnv("NODE_ENV", "development");
		const evaluation = registerWorkspaceEvaluation(
			"conversation",
			"improved",
			"source",
		);
		for (const args of [
			{ app_id: "", appId: "source" },
			{ app_id: "   ", appId: "source" },
		]) {
			expect(
				workspaceEvaluationMutationError(evaluation, "flowpilot_board", args),
			).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		}
		expect(
			workspaceEvaluationTargetError(
				evaluation,
				"flowpilot_board",
				{},
				"source",
			),
		).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		expect(
			workspaceEvaluationTargetError(
				evaluation,
				"flowpilot_board",
				{ app_id: "destination" },
				"source",
			),
		).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		expect(
			workspaceEvaluationTargetError(
				evaluation,
				"inspect_app",
				{},
				"prior-target",
			),
		).toMatchObject({ code: "EVALUATION_APP_OUT_OF_SCOPE" });
		for (const [tool, args] of [
			["create_app", {}],
			["project_scout", {}],
			["list_apps", {}],
			["describe_app_interface", {}],
			["flowpilot_board", { mode: "explain" }],
			["app_build", { operation: "status" }],
		] as const) {
			expect(
				workspaceEvaluationTargetError(evaluation, tool, args, "source"),
			).toBeUndefined();
		}
		expect(
			workspaceEvaluationTargetError(
				evaluation,
				"list_apps",
				{},
				"prior-target",
			),
		).toBeUndefined();
		expect(
			workspaceEvaluationTargetError(
				undefined,
				"flowpilot_board",
				{},
				"source",
			),
		).toBeUndefined();
	});
	it("admits only host-registered destinations and observes membership revocations on every read", async () => {
		vi.stubEnv("NODE_ENV", "development");
		const evaluation = registerWorkspaceEvaluation(
			"conversation",
			"improved",
			"source",
		);
		expect(workspaceEvaluationAllowsApp(evaluation, "destination")).toBe(false);
		registerWorkspaceEvaluationDestination(evaluation, "destination");
		expect(workspaceEvaluationAllowsApp(evaluation, "destination")).toBe(true);
		expect(workspaceEvaluationAllowsApp(evaluation, "prior-target")).toBe(
			false,
		);
		expect(
			workspaceEvaluationTargetError(
				evaluation,
				"flowpilot_board",
				{},
				"destination",
			),
		).toBeUndefined();
		let calls = 0;
		vi.spyOn(evaluation.session, "search").mockImplementation(
			async (_backend, _args, scope) => {
				expect(scope.scopedAppId).toBeUndefined();
				expect([...(await scope.getProfileAppIds())]).toEqual([
					"source",
					"destination",
				]);
				expect([...(await scope.getProfileAppIds())]).toEqual([]);
				return { status: "ok", hits: [] };
			},
		);
		await dispatchWorkspaceEvaluationRead(
			evaluation,
			"search_workspace",
			{} as IBackendState,
			{ query: "policy" },
			{
				scopedAppId: "prior-target",
				getProfileAppIds: async () =>
					new Set(
						calls++ === 0 ? ["source", "destination", "prior-target"] : [],
					),
			},
		);
		expect(evaluation.calls).toHaveLength(1);
	});
	it.each([false, true])(
		"records the exact source target from a destination-scoped read, including stale reads: %s",
		async (stale) => {
			vi.stubEnv("NODE_ENV", "development");
			const evaluation = registerWorkspaceEvaluation(
				"conversation",
				"improved",
				"source",
			);
			registerWorkspaceEvaluationDestination(evaluation, "destination");
			const source = "function routeRecord() {}";
			const getFlowScriptAuthoritative = vi.fn(async () => source);
			const backend = {
				boardState: { getFlowScriptAuthoritative },
			} as unknown as IBackendState;
			const result = await dispatchWorkspaceEvaluationRead(
				evaluation,
				"read_symbol",
				backend,
				{
					resource_id: workspaceResourceId({
						kind: "workflow",
						app_id: "source",
						board_id: "policy-board",
						id: "routeRecord",
					}),
					revision: stale ? "0".repeat(64) : await workspaceRevision(source),
				},
				{
					scopedAppId: "destination",
					getProfileAppIds: async () => new Set(["source", "destination"]),
				},
			);
			expect(result.status).toBe(stale ? "stale" : "ok");
			expect(evaluation.calls).toHaveLength(1);
			expect(evaluation.calls[0]).toMatchObject({
				tool: "read_symbol",
				appId: "source",
				status: stale ? "stale" : "ok",
			});
			expect(getFlowScriptAuthoritative).toHaveBeenCalledWith(
				"source",
				"policy-board",
				undefined,
				true,
			);
		},
	);

	it("admits a writable preprovisioned build app without inventing a source fixture", async () => {
		vi.stubEnv("NODE_ENV", "development");
		const evaluation = registerWorkspaceBuildEvaluation(
			"conversation",
			"build-app",
		);
		expect(getWorkspaceEvaluation("conversation")).toBe(evaluation);
		expect(evaluation).not.toHaveProperty("sourceAppId");
		expect(evaluation.mode).toBe("improved");
		expect([...evaluation.destinationAppIds]).toEqual(["build-app"]);
		for (const tool of [
			"flowpilot_board",
			"flowpilot_widget",
			"database_tool",
		]) {
			expect(
				workspaceEvaluationTargetError(evaluation, tool, {
					app_id: "build-app",
				}),
			).toBeUndefined();
			expect(
				workspaceEvaluationTargetError(evaluation, tool, {}, "prior-app"),
			).toMatchObject({ code: "EVALUATION_APP_OUT_OF_SCOPE" });
		}
		for (const tool of ["create_app", "upsert_event"]) {
			expect(
				workspaceEvaluationTargetError(evaluation, tool, {
					app_id: "build-app",
				}),
			).toMatchObject({ code: "EVALUATION_HOST_PROVISIONED" });
		}
		expect(
			workspaceEvaluationMutationError(evaluation, "upsert_event", {}),
		).toBeUndefined();
		const getBoardSummariesAuthoritative = vi.fn(async () => [
			{ id: "board", name: "Board" },
		]);
		const getFlowScriptAuthoritative = vi.fn(
			async () => "function invoice() {}",
		);
		const backend = {
			boardState: {
				getBoardSummariesAuthoritative,
				getFlowScriptAuthoritative,
			},
		} as unknown as IBackendState;
		const result = await dispatchWorkspaceEvaluationRead(
			evaluation,
			"search_workspace",
			backend,
			{ query: "invoice", kinds: ["workflow"] },
			{
				getProfileAppIds: async () => new Set(["prior-app", "build-app"]),
			},
		);
		expect(result.status).toBe("ok");
		expect(result.coverage).toMatchObject({
			app_ids: ["build-app"],
			complete: true,
		});
		expect(getBoardSummariesAuthoritative).toHaveBeenCalledExactlyOnceWith(
			"build-app",
		);
		expect(getFlowScriptAuthoritative).toHaveBeenCalledExactlyOnceWith(
			"build-app",
			"board",
			undefined,
			true,
		);
		expect(evaluation.calls).toHaveLength(1);
	});

	it("keeps build registration development-only and shares duplicate-registration guards with comparisons", () => {
		vi.stubEnv("NODE_ENV", "production");
		expect(() =>
			registerWorkspaceBuildEvaluation("conversation", "build-app"),
		).toThrow("development");
		vi.stubEnv("NODE_ENV", "development");
		expect(() => registerWorkspaceBuildEvaluation("conversation", " ")).toThrow(
			"app ID",
		);
		registerWorkspaceBuildEvaluation("conversation", "build-app");
		expect(() =>
			registerWorkspaceBuildEvaluation("conversation", "another-app"),
		).toThrow("already");
		expect(() =>
			registerWorkspaceEvaluation("conversation", "baseline", "source"),
		).toThrow("already");
		unregisterWorkspaceEvaluation("conversation");
		const comparison = registerWorkspaceEvaluation(
			"conversation",
			"baseline",
			"source",
		);
		expect(
			workspaceEvaluationTargetError(comparison, "upsert_event", {
				app_id: "source",
			}),
		).toMatchObject({ code: "EVALUATION_SOURCE_READ_ONLY" });
		expect(() =>
			registerWorkspaceBuildEvaluation("conversation", "build-app"),
		).toThrow("already");
	});

	it("keeps the frozen baseline's conjunctive matching", () => {
		const docs = [
			{
				resource_id: "doc",
				revision: "r",
				target: { kind: "doc", id: "doc" },
				title: "Queue",
				content: "refund billing",
			},
		] satisfies WorkspaceDocument[];
		expect(baseline(docs, "refund").hits).toHaveLength(1);
		expect(baseline(docs, "refund rendering").hits).toHaveLength(0);
	});
});
