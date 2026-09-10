import { describe, expect, test, vi } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import {
	type AppBuildFoundation,
	provisionAppBuildFoundation,
} from "./resource-foundation";

const contract: AppBuildFoundation = {
	app_id: "app",
	boards: [{ id: "owned-board", name: "Intake" }],
	tables: [
		{
			name: "tickets",
			columns: [{ name: "summary", type: "string", nullable: false }],
		},
	],
};

function fixture() {
	const calls: string[] = [];
	const tables = new Map<string, unknown>();
	const boards = new Map<
		string,
		{ id: string; name: string; nodes: Record<string, unknown> }
	>();
	let status = "Inactive";
	let persistTable = true;
	const backend = {
		appState: { getAppAuthoritative: async () => ({ status }) },
		dbState: {
			listTablesAuthoritative: async () => [...tables.keys()],
			getSchemaAuthoritative: async (_app: string, name: string) =>
				tables.get(name),
			createTable: vi.fn(async (_app: string, name: string) => {
				calls.push("create-table");
				if (persistTable)
					tables.set(name, {
						fields: [{ name: "summary", data_type: "Utf8", nullable: false }],
					});
				return { created: true };
			}),
		},
		boardState: {
			getBoardSummariesAuthoritative: async () => [...boards.values()],
			getBoardAuthoritative: async (_app: string, id: string) => {
				calls.push("read-board");
				return boards.get(id);
			},
			upsertBoard: vi.fn(async (_app: string, id: string, name: string) => {
				calls.push("create-board");
				boards.set(id, { id, name, nodes: {} });
			}),
			getFlowScriptAuthoritative: vi.fn(async () => {
				calls.push("open-native-board");
				return "";
			}),
		},
	} as unknown as IBackendState;
	return {
		backend,
		calls,
		tables,
		boards,
		activate: () => {
			status = "Active";
		},
		loseTableWrite: () => {
			persistTable = false;
		},
	};
}

describe("host app foundation", () => {
	test("persists typed tables before board shells and opens the compiler handle", async () => {
		const f = fixture();
		const evidence = await provisionAppBuildFoundation(f.backend, contract, {
			assertActive() {},
		});
		expect(f.calls).toEqual([
			"create-table",
			"create-board",
			"read-board",
			"open-native-board",
		]);
		expect(evidence.tables[0]).toMatchObject({
			name: "tickets",
			created: true,
		});
		expect(evidence.boards[0]).toEqual({
			id: "owned-board",
			name: "Intake",
			reused: false,
		});
	});
	test("repeated setup verifies existing resources without replacing rows or workflow content", async () => {
		const f = fixture();
		await provisionAppBuildFoundation(f.backend, contract, {
			assertActive() {},
		});
		const existing = f.boards.get("owned-board");
		if (!existing) throw new Error("Expected the provisioned board.");
		existing.nodes.handler = { name: "events_generic" };
		const evidence = await provisionAppBuildFoundation(f.backend, contract, {
			assertActive() {},
		});
		expect(f.backend.dbState.createTable).toHaveBeenCalledTimes(1);
		expect(f.backend.boardState.upsertBoard).toHaveBeenCalledTimes(1);
		expect(f.boards.get("owned-board")?.nodes).toHaveProperty("handler");
		expect(evidence.boards[0].reused).toBe(true);
	});
	test("schema drift prevents any additional setup writes", async () => {
		const f = fixture();
		f.tables.set("tickets", {
			fields: [{ name: "summary", data_type: "Int64", nullable: false }],
		});
		await expect(
			provisionAppBuildFoundation(f.backend, contract, { assertActive() {} }),
		).rejects.toThrow("different type");
		expect(f.backend.dbState.createTable).not.toHaveBeenCalled();
		expect(f.backend.boardState.upsertBoard).not.toHaveBeenCalled();
	});
	test("a successful create reply without authoritative persistence cannot provision boards", async () => {
		const f = fixture();
		f.loseTableWrite();
		await expect(
			provisionAppBuildFoundation(f.backend, contract, { assertActive() {} }),
		).rejects.toThrow("not persisted");
		expect(f.backend.boardState.upsertBoard).not.toHaveBeenCalled();
	});
	test("schema inventory errors stay errors without creating a replacement table", async () => {
		const f = fixture();
		f.backend.dbState.listTablesAuthoritative = async () => {
			throw new Error("offline");
		};
		await expect(
			provisionAppBuildFoundation(f.backend, contract, { assertActive() {} }),
		).rejects.toThrow("offline");
		expect(f.backend.dbState.createTable).not.toHaveBeenCalled();
	});
	test("rejects a live app and duplicate IDs before mutation", async () => {
		const f = fixture();
		f.activate();
		await expect(
			provisionAppBuildFoundation(f.backend, contract, { assertActive() {} }),
		).rejects.toThrow("inactive");
		await expect(
			provisionAppBuildFoundation(
				f.backend,
				{ ...contract, boards: [...contract.boards, ...contract.boards] },
				{ assertActive() {} },
			),
		).rejects.toThrow("Duplicate");
		expect(f.calls).toEqual([]);
	});
	test("a failed canonical read cannot produce setup evidence", async () => {
		const f = fixture();
		f.backend.boardState.getFlowScriptAuthoritative = async () => {
			throw new Error("board handle unavailable");
		};
		await expect(
			provisionAppBuildFoundation(f.backend, contract, { assertActive() {} }),
		).rejects.toThrow("board handle unavailable");
	});
});
