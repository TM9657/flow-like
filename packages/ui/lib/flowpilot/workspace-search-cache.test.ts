import { describe, expect, it, vi } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import type { WorkspaceScope } from "./workspace-content";
import { createWorkspaceIndex } from "./workspace-ranker";
import { WorkspaceSearchSession } from "./workspace-search";

function harness(options: { cacheEnabled?: boolean } = {}) {
	const sources: Record<string, string> = {
		billing: 'function invoice() { return "ledger" }',
		shipping: 'function shipment() { return "tracking" }',
		users: 'function account() { return "profile" }',
	};
	let now = 1_000;
	let profileId = "profile-1";
	const appIds = new Set(["app-1", "app-2"]);
	const getProfileAppIds = vi.fn(async () => new Set(appIds));
	const getProfileIdentity = vi.fn(async () => profileId);
	const getBoardSummariesAuthoritative = vi.fn(async () =>
		Object.keys(sources).map((id) => ({ id, name: id })),
	);
	const getFlowScriptAuthoritative = vi.fn(
		async (_appId: string, boardId: string) => {
			if (!sources[boardId]) throw new Error("Missing board");
			return sources[boardId];
		},
	);
	const backend = {
		boardState: { getBoardSummariesAuthoritative, getFlowScriptAuthoritative },
	} as unknown as IBackendState;
	const scope: WorkspaceScope = { getProfileAppIds, getProfileIdentity };
	const createIndex = vi.fn(createWorkspaceIndex);
	const session = new WorkspaceSearchSession(() => now, {
		...options,
		createIndex,
	});
	const search = (
		args: Record<string, unknown> = {},
		access: WorkspaceScope = scope,
		source = backend,
	) =>
		session.search(
			source,
			{ query: "invoice", app_id: "app-1", kinds: ["workflow"], ...args },
			access,
		);
	return {
		sources,
		appIds,
		scope,
		backend,
		session,
		search,
		createIndex,
		getProfileAppIds,
		getProfileIdentity,
		getBoardSummariesAuthoritative,
		getFlowScriptAuthoritative,
		advance: (ms: number) => {
			now += ms;
		},
		profile: (id: string) => {
			profileId = id;
		},
	};
}

function freshness(result: Record<string, unknown>) {
	expect(result.status).toBe("ok");
	return result.freshness;
}

describe("workspace index reuse", () => {
	it("loads the corpus once for different queries and rereads only returned cached hits", async () => {
		const test = harness();
		expect(freshness(await test.search())).toMatchObject({
			cache: "miss",
			hits_revalidated: 0,
			coverage_basis: "observed_snapshot",
		});
		test.advance(50);
		const next = await test.search({ query: "tracking" });
		expect(freshness(next)).toMatchObject({
			cache: "hit",
			hits_revalidated: 1,
			corpus_age_ms: 50,
			inventory_revalidated: false,
		});
		expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(1);
		expect(test.getFlowScriptAuthoritative).toHaveBeenCalledTimes(4);
		expect(test.getFlowScriptAuthoritative).toHaveBeenLastCalledWith(
			"app-1",
			"shipping",
			undefined,
			true,
		);
		expect(test.createIndex).toHaveBeenCalledTimes(1);
	});

	it("shares concurrent loads of the same corpus without rebuilding its index", async () => {
		const test = harness();
		let release: (() => void) | undefined;
		const blocked = new Promise<void>((resolve) => {
			release = resolve;
		});
		test.getBoardSummariesAuthoritative.mockImplementation(async () => {
			await blocked;
			return Object.keys(test.sources).map((id) => ({ id, name: id }));
		});
		const first = test.search();
		const second = test.search({ query: "shipment" });
		await vi.waitFor(() =>
			expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(1),
		);
		release?.();
		const results = await Promise.all([first, second]);
		expect(
			results
				.map((result) => (freshness(result) as { cache: string }).cache)
				.sort(),
		).toEqual(["miss", "shared_load"]);
		expect(test.createIndex).toHaveBeenCalledTimes(1);
		expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(1);
	});

	it("expires corpora from observation time even when queries keep using them", async () => {
		const test = harness();
		await test.search();
		test.advance(119_000);
		expect(freshness(await test.search())).toMatchObject({
			cache: "hit",
			corpus_age_ms: 119_000,
		});
		test.advance(1_000);
		expect(freshness(await test.search())).toMatchObject({
			cache: "miss",
			corpus_age_ms: 0,
		});
		expect(test.createIndex).toHaveBeenCalledTimes(2);
	});

	it("can disable reuse and inject a frozen index for a paired evaluation", async () => {
		const test = harness({ cacheEnabled: false });
		await test.search();
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
		expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(2);
		expect(test.getFlowScriptAuthoritative).toHaveBeenCalledTimes(6);
		expect(test.createIndex).toHaveBeenCalledTimes(2);
	});

	it("labels an absent result as observed coverage and discovers additions after clear", async () => {
		const test = harness();
		await test.search();
		test.sources.refunds = "function refund() {}";
		const cached = await test.search({ query: "refund" });
		expect(cached.hits).toEqual([]);
		expect(freshness(cached)).toMatchObject({
			cache: "hit",
			inventory_revalidated: false,
		});
		test.session.clear();
		const fresh = await test.search({ query: "refund" });
		expect(fresh.hits).toHaveLength(1);
		expect(freshness(fresh)).toMatchObject({ cache: "miss" });
	});
	it("bounds retained serialized source bytes across otherwise distinct corpora", async () => {
		const test = harness();
		for (const key of Object.keys(test.sources)) delete test.sources[key];
		for (let i = 0; i < 6; i++)
			test.sources[`large${i}`] =
				`function invoice${i}() { return "${"x".repeat(900_000)}" }`;
		// Isolate retention from ranker memory and scoring in this large-source case.
		test.createIndex.mockImplementation(() => ({
			search: () => ({ hits: [], mode: "test" }),
		}));
		await test.search({ app_id: "app-1" });
		await test.search({ app_id: "app-2" });
		expect(freshness(await test.search({ app_id: "app-1" }))).toMatchObject({
			cache: "miss",
		});
		expect(test.createIndex).toHaveBeenCalledTimes(3);
	});

	it("reloads transient inventory failures on the next query", async () => {
		const test = harness();
		test.getBoardSummariesAuthoritative.mockRejectedValueOnce(
			new Error("temporarily offline"),
		);
		const partial = await test.search();
		expect(partial.coverage).toMatchObject({
			complete: false,
			issues: [expect.objectContaining({ code: "INVENTORY_UNREADABLE" })],
		});
		const recovered = await test.search();
		expect(recovered.hits).toHaveLength(1);
		expect(freshness(recovered)).toMatchObject({ cache: "miss" });
	});

	it("clears a failed index build instead of retaining a rejected shared load", async () => {
		const test = harness();
		test.createIndex.mockImplementationOnce(() => {
			throw new Error("index failed");
		});
		expect(await test.search()).toMatchObject({
			code: "WORKSPACE_NOT_READABLE",
		});
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
		expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(2);
	});
});

describe("workspace cache access and freshness", () => {
	it("rejects changed cached snippets, evicts their corpus and returns fresh evidence on retry", async () => {
		const test = harness();
		await test.search();
		test.sources.billing = 'function invoice() { return "new ledger" }';
		const stale = await test.search();
		expect(stale).toMatchObject({
			status: "stale",
			code: "WORKSPACE_SNAPSHOT_CHANGED",
		});
		expect(stale).not.toHaveProperty("hits");
		const fresh = await test.search();
		expect(freshness(fresh)).toMatchObject({ cache: "miss" });
		expect(JSON.stringify(fresh.hits)).toContain("new ledger");
		expect(test.createIndex).toHaveBeenCalledTimes(2);
	});

	it("invalidates retained discovery immediately when an exact read observes a changed revision", async () => {
		const test = harness();
		const first = await test.search();
		const hit = (first.hits as Record<string, unknown>[])[0];
		test.sources.billing = 'function invoice() { return "changed" }';
		expect(
			await test.session.readSymbol(test.backend, hit, test.scope),
		).toMatchObject({ status: "stale", code: "WORKSPACE_REVISION_CHANGED" });
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
	});

	it("never reuses a cursor or index on another backend instance", async () => {
		const test = harness();
		test.sources.billing = "function invoiceA() {}\nfunction invoiceB() {}";
		const first = await test.search({ limit: 1 });
		const otherBackend = { ...test.backend };
		expect(
			await test.search(
				{ cursor: first.next_cursor },
				test.scope,
				otherBackend,
			),
		).toMatchObject({ code: "WORKSPACE_CURSOR_INVALID" });
		expect(
			freshness(await test.search({}, test.scope, otherBackend)),
		).toMatchObject({ cache: "miss" });
	});

	it("purges indexes when profile identity changes even if both profiles list identical app IDs", async () => {
		const test = harness();
		await test.search();
		test.profile("profile-2");
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
		test.profile("profile-1");
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
		expect(test.createIndex).toHaveBeenCalledTimes(3);
	});

	it("purges on access-read failure and membership changes without returning cached content", async () => {
		const test = harness();
		await test.search();
		test.getProfileAppIds.mockRejectedValueOnce(new Error("offline"));
		expect(await test.search()).toMatchObject({
			code: "WORKSPACE_PROFILE_UNREADABLE",
		});
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
		test.appIds.delete("app-1");
		const denied = await test.search();
		expect(denied).toMatchObject({ code: "WORKSPACE_NOT_READABLE" });
		expect(denied).not.toHaveProperty("hits");
	});

	it("rejects cached hits when the backend revokes source access while profile membership remains", async () => {
		const test = harness();
		await test.search();
		test.getFlowScriptAuthoritative.mockRejectedValueOnce(new Error("denied"));
		const denied = await test.search();
		expect(denied).toMatchObject({
			status: "stale",
			code: "WORKSPACE_SNAPSHOT_CHANGED",
		});
		expect(denied).not.toHaveProperty("hits");
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
	});

	it("keeps current app and resource selection distinct, bounded to two retained corpora", async () => {
		const test = harness();
		const firstScope = { ...test.scope, scopedAppId: "app-1" };
		const secondScope = { ...test.scope, scopedAppId: "app-2" };
		await test.search({}, firstScope);
		expect(freshness(await test.search({}, secondScope))).toMatchObject({
			cache: "miss",
		});
		expect(
			freshness(await test.search({ board_id: "billing" }, secondScope)),
		).toMatchObject({ cache: "miss" });
		expect(freshness(await test.search({}, firstScope))).toMatchObject({
			cache: "miss",
		});
		expect(test.createIndex).toHaveBeenCalledTimes(4);
	});

	it("does not let a cleared in-flight load repopulate retained state", async () => {
		const test = harness();
		let release: (() => void) | undefined;
		const blocked = new Promise<void>((resolve) => {
			release = resolve;
		});
		test.getBoardSummariesAuthoritative.mockImplementationOnce(async () => {
			await blocked;
			return [{ id: "billing", name: "billing" }];
		});
		const pending = test.search();
		await vi.waitFor(() =>
			expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(1),
		);
		test.session.clear();
		release?.();
		expect(await pending).toMatchObject({
			status: "stale",
			code: "WORKSPACE_SNAPSHOT_CHANGED",
		});
		expect(freshness(await test.search())).toMatchObject({ cache: "miss" });
	});

	it("caps concurrent authoritative cached-result reads at four", async () => {
		const test = harness();
		for (let i = 0; i < 10; i++)
			test.sources[`invoices${i}`] = `function invoice${i}() {}`;
		await test.search();
		let active = 0;
		let peak = 0;
		test.getFlowScriptAuthoritative.mockImplementation(
			async (_appId, boardId) => {
				active++;
				peak = Math.max(peak, active);
				await new Promise((resolve) => setTimeout(resolve, 1));
				active--;
				return test.sources[boardId];
			},
		);
		expect(freshness(await test.search())).toMatchObject({
			cache: "hit",
			hits_revalidated: 10,
		});
		expect(peak).toBe(4);
	});
	it("validates several cached symbols from one board with one authoritative read", async () => {
		const test = harness();
		test.sources.billing = Array.from(
			{ length: 10 },
			(_, i) => `function invoice${i}() {}`,
		).join("\n");
		await test.search();
		test.getFlowScriptAuthoritative.mockClear();
		expect(freshness(await test.search())).toMatchObject({
			cache: "hit",
			hits_revalidated: 10,
			validation_reads: 1,
		});
		expect(test.getFlowScriptAuthoritative).toHaveBeenCalledTimes(1);
	});

	it.each([false, true])(
		"caps concurrent corpus loads across selections even after clear: %s",
		async (clear) => {
			const test = harness();
			let release: (() => void) | undefined;
			const blocked = new Promise<void>((resolve) => {
				release = resolve;
			});
			test.getFlowScriptAuthoritative.mockImplementation(
				async (_appId, boardId) => {
					await blocked;
					return test.sources[boardId];
				},
			);
			const first = test.search({ board_id: "billing" });
			const second = test.search({ board_id: "shipping" });
			await vi.waitFor(() =>
				expect(test.getFlowScriptAuthoritative).toHaveBeenCalledTimes(2),
			);
			if (clear) test.session.clear();
			expect(await test.search({ board_id: "users" })).toMatchObject({
				code: "WORKSPACE_SEARCH_BUSY",
			});
			expect(test.getFlowScriptAuthoritative).toHaveBeenCalledTimes(2);
			release?.();
			for (const result of await Promise.all([first, second]))
				expect(result.status).toBe(clear ? "stale" : "ok");
		},
	);
});
