import { describe, expect, test } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import { compileAppSpec } from "./compiler";
import { createAppBuildHostAdapter } from "./host-adapter";
import {
	createBuild,
	markResourcesApplying,
	recordResourceResults,
} from "./state";
import { exampleAppSpec } from "./test-fixtures";

function fixture(options: { table?: boolean; badSchema?: boolean } = {}) {
	const spec = exampleAppSpec();
	if (options.table)
		spec.resources.push({
			key: "tickets",
			kind: "table",
			depends_on: [],
			requirement_ids: ["capture_issue"],
			config: {
				name: "tickets",
				columns: [{ name: "summary", type: "string" }],
			},
		});
	const plan = compileAppSpec(spec, { app_id: "app", build_id: "build" });
	const resource = plan.resources.find((resource) => resource.kind === "board");
	if (!resource) throw new Error("Expected the fixture board.");
	let source = "flow issue_flow {}";
	let exists = false;
	let reconciliation = true;
	let active = false;
	let activateDuringGeneration = false;
	let mutations = 0;
	let tableExists = false;
	const delegated: Record<string, unknown>[] = [];
	const backend = {
		appState: {
			getApp: async () => ({ status: active ? "Active" : "Inactive" }),
			getAppAuthoritative: async () => ({
				status: active ? "Active" : "Inactive",
			}),
		},
		boardState: {
			upsertBoard: async () => {
				exists = true;
			},
			getBoardSummariesAuthoritative: async () =>
				exists ? [{ id: resource.physical_id }] : [],
			getBoardAuthoritative: async () => ({
				id: resource.physical_id,
				nodes: { entry: { id: "entry", name: "events_simple", pins: {} } },
			}),
			getFlowScriptAuthoritative: async () => source,
			checkFlowScriptReconcile: async () => ({
				parse_valid: reconciliation,
				reconcile_valid: reconciliation,
				diagnostics: [],
			}),
		},
		eventState: { getEventsAuthoritative: async () => [] },
		dbState: {
			listTablesAuthoritative: async () => (tableExists ? ["tickets"] : []),
			createTable: async () => {
				tableExists = true;
				return { created: true };
			},
			getSchemaAuthoritative: async () => ({
				fields: [
					{ name: "summary", data_type: options.badSchema ? "Int64" : "Utf8" },
				],
			}),
		},
	} as unknown as IBackendState;
	const host = createAppBuildHostAdapter(backend, {
		assertActive() {},
		referenceApp() {},
		generateWidget: async () => [],
		delegate: async (_tool, args) => {
			delegated.push({ ...args, tableExists });
			mutations++;
			exists = true;
			if (activateDuringGeneration) active = true;
			return { status: "ok" };
		},
	});
	const build = markResourcesApplying(
		createBuild(spec, { app_id: "app", build_id: "build" }),
		[resource.key],
		{
			operation_id: "operation",
			operation_owner_id: "owner",
			lease_expires_at_ms: Date.now() + 60_000,
		},
	);
	return {
		plan,
		resource,
		build,
		host,
		mutations: () => mutations,
		delegated,
		changeSource() {
			source += "\nchanged";
		},
		makeExist() {
			exists = true;
		},
		failCompiler() {
			reconciliation = false;
		},
		activate() {
			active = true;
		},
		activateDuringGeneration() {
			activateDuringGeneration = true;
		},
	};
}

describe("interactive app build host", () => {
	test("provisions undeclared table dependencies before the board specialist starts", async () => {
		const f = fixture({ table: true });
		expect((await f.host.applyResource(f)).status).toBe("applied");
		expect(f.delegated[0]).toMatchObject({
			tableExists: true,
			board_id: f.resource.physical_id,
			create_new_board: false,
		});
	});
	test("a malformed provisioned schema prevents specialist generation", async () => {
		const f = fixture({ table: true, badSchema: true });
		expect(await f.host.applyResource(f)).toMatchObject({
			status: "unknown",
			message: expect.stringContaining("different type"),
		});
		expect(f.mutations()).toBe(0);
	});
	test("cannot certify writes when the app becomes active during generation", async () => {
		const f = fixture();
		f.activateDuringGeneration();
		expect(await f.host.applyResource(f)).toMatchObject({ status: "unknown" });
		expect(f.mutations()).toBe(1);
	});
	test("records actual artifact hashes, distinct from desired spec hashes", async () => {
		const f = fixture();
		const result = await f.host.applyResource(f);
		expect(result.status).toBe("applied");
		expect(result.observed_fingerprint).not.toBe(result.desired_fingerprint);
		const build = recordResourceResults(f.build, { [f.resource.key]: result });
		expect((await f.host.inspectResource({ ...f, build })).status).toBe(
			"matches",
		);
		f.changeSource();
		expect((await f.host.inspectResource({ ...f, build })).status).toBe(
			"drifted",
		);
	});
	test("mere resource existence cannot certify an interrupted operation", async () => {
		const f = fixture();
		f.makeExist();
		expect((await f.host.inspectResource(f)).status).toBe("unknown");
		expect(f.mutations()).toBe(0);
	});
	test("a failed compiler check cannot become an applied receipt", async () => {
		const f = fixture();
		f.failCompiler();
		expect((await f.host.applyResource(f)).status).toBe("partial");
	});
	test("resource generation refuses a live app", async () => {
		const f = fixture();
		f.activate();
		expect((await f.host.applyResource(f)).status).toBe("unknown");
		expect(f.mutations()).toBe(0);
	});
});
