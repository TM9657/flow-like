import { beforeEach, describe, expect, it, vi } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import type { WorkspaceScope } from "./workspace-content";
import {
	type WorkspaceDocument,
	type WorkspaceTarget,
	boundedWorkspaceText,
	jsonBytes,
	parseWorkspaceResourceId,
	workspaceResourceId,
	workspaceRevision,
} from "./workspace-resource";
import {
	WorkspaceSearchSession,
	rankWorkspaceDocuments,
	readWorkspaceSymbol,
} from "./workspace-search";

const { loadDocs } = vi.hoisted(() => ({ loadDocs: vi.fn() }));
vi.mock("./workspace-docs", () => ({ loadFlowPilotWorkspaceDocs: loadDocs }));

type Hit = {
	resource_id: string;
	revision: string;
	title: string;
	symbol?: string;
	snippet: string;
	matched_fields: string[];
};

function hits(response: Record<string, unknown>): Hit[] {
	expect(response.status).toBe("ok");
	return response.hits as Hit[];
}

function harness(
	initial: Record<string, string>,
	options: { scopedAppId?: string; now?: () => number } = {},
) {
	const sources = { ...initial };
	const profile = new Set(["app-1"]);
	const getProfileAppIds = vi.fn(async () => new Set(profile));
	const getBoardSummariesAuthoritative = vi.fn(async () =>
		Object.keys(sources).map((id) => ({ id, name: id })),
	);
	const getFlowScriptAuthoritative = vi.fn(
		async (_appId: string, boardId: string) => {
			if (!(boardId in sources)) throw new Error("Board disappeared.");
			return sources[boardId];
		},
	);
	const backend = {
		boardState: { getBoardSummariesAuthoritative, getFlowScriptAuthoritative },
	} as unknown as IBackendState;
	const scope: WorkspaceScope = {
		getProfileAppIds,
		scopedAppId: options.scopedAppId,
	};
	const session = new WorkspaceSearchSession(options.now);
	const search = (args: Record<string, unknown> = {}) =>
		session.search(
			backend,
			{ query: "invoice", app_id: "app-1", kinds: ["workflow"], ...args },
			scope,
		);
	return {
		sources,
		profile,
		getProfileAppIds,
		getBoardSummariesAuthoritative,
		getFlowScriptAuthoritative,
		backend,
		scope,
		session,
		search,
	};
}

async function document(
	target: WorkspaceTarget,
	title: string,
	content: string,
	extra: Partial<WorkspaceDocument> = {},
): Promise<WorkspaceDocument> {
	return {
		target,
		resource_id: workspaceResourceId(target),
		title,
		content,
		revision: await workspaceRevision(content),
		...extra,
	};
}

beforeEach(() => {
	loadDocs.mockReset();
	loadDocs.mockResolvedValue({
		schema_version: 1,
		revision: "a".repeat(64),
		coverage: {
			mode: "bundled_selected_docs",
			complete: true,
			source_paths: ["dev/workflows.mdx"],
			excluded: ["Runtime data"],
		},
		entries: [
			{
				kind: "doc",
				resource_id: "workflow-guidance",
				title: "Workflow guidance",
				revision: "b".repeat(64),
				content_revision: "c".repeat(64),
				source_path: "dev/workflows.mdx",
				section_id: "workflow-guidance",
				content: "Reuse an invoice helper before creating a new workflow.",
				line_start: 12,
				line_end: 14,
			},
		],
	});
});

describe("FlowPilot workspace full-text discovery", () => {
	it("finds body-only evidence and reads precisely the discovered helper", async () => {
		const source = `function unrelated() { log::info({ message: "neighbor implementation" }) }
module billing {
    // Retrieves the persisted records.
    function loadRecords(): (rows: Struct[]) { //@l:load-records
        const rows = db::query({ sql: "SELECT invoice_id FROM accounting" })
        return rows
    }
    function neighbor() { log::info({ message: "must stay outside the read" }) }
}
`;
		const test = harness({ "board-1": source });
		const response = await test.search({ query: "invoice_id accounting" });
		const found = hits(response);
		expect(found).toHaveLength(1);
		expect(found[0]).toMatchObject({
			symbol: "billing::loadRecords",
			matched_fields: ["content"],
		});
		expect(found[0].snippet).toContain("SELECT invoice_id FROM accounting");
		expect(response.coverage).toMatchObject({
			complete: true,
			indexed_resources: 3,
		});
		const read = await readWorkspaceSymbol(test.backend, found[0], test.scope);
		expect(read).toMatchObject({
			status: "ok",
			symbol: "billing::loadRecords",
			anchor_id: "load-records",
			start_line: 3,
			end_line: 7,
			next_offset: null,
			content_is_untrusted: true,
		});
		expect(read.content).toBe(`// Retrieves the persisted records.
    function loadRecords(): (rows: Struct[]) { //@l:load-records
        const rows = db::query({ sql: "SELECT invoice_id FROM accounting" })
        return rows
    }`);
		expect(String(read.content)).not.toContain("neighbor");
		expect(test.getFlowScriptAuthoritative).toHaveBeenLastCalledWith(
			"app-1",
			"board-1",
			undefined,
			true,
		);
	});

	it("rejects a stale source revision and allows a fresh discovery/read", async () => {
		const test = harness({ "board-1": 'function invoice() { return "old" }' });
		const original = hits(await test.search())[0];
		test.sources["board-1"] = 'function invoice() { return "new" }';
		const stale = await test.session.readSymbol(
			test.backend,
			original,
			test.scope,
		);
		expect(stale).toMatchObject({
			status: "stale",
			code: "WORKSPACE_REVISION_CHANGED",
		});
		expect(stale).not.toHaveProperty("content");
		const fresh = hits(await test.search())[0];
		expect(fresh.revision).not.toBe(original.revision);
		expect(fresh.resource_id).toBe(original.resource_id);
		expect(
			await readWorkspaceSymbol(test.backend, fresh, test.scope),
		).toMatchObject({
			status: "ok",
			content: 'function invoice() { return "new" }',
		});
	});

	it("blocks unrelated app discovery and exact reads before backend calls", async () => {
		const test = harness({ "board-1": "function invoice() {}" });
		const search = await test.search({ app_id: "unrelated-app" });
		expect(search).toMatchObject({
			status: "error",
			code: "WORKSPACE_NOT_READABLE",
		});
		const read = await readWorkspaceSymbol(
			test.backend,
			{
				resource_id: workspaceResourceId({
					kind: "workflow",
					app_id: "unrelated-app",
					board_id: "board-1",
					id: "invoice",
				}),
				revision: "a".repeat(64),
			},
			test.scope,
		);
		expect(read).toMatchObject({
			status: "error",
			code: "WORKSPACE_RESOURCE_UNREADABLE",
		});
		expect(test.getBoardSummariesAuthoritative).not.toHaveBeenCalled();
		expect(test.getFlowScriptAuthoritative).not.toHaveBeenCalled();
	});

	it("uses the explicitly scoped app when no app filter is supplied", async () => {
		const test = harness(
			{ "board-1": "function invoice() {}" },
			{ scopedAppId: "app-current" },
		);
		const response = await test.session.search(
			test.backend,
			{ query: "invoice", kinds: ["workflow"] },
			test.scope,
		);
		expect(hits(response)[0].resource_id).toBe(
			workspaceResourceId({
				kind: "workflow",
				app_id: "app-current",
				board_id: "board-1",
				id: "invoice",
			}),
		);
		expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledWith(
			"app-current",
		);
	});

	it("keeps partial or malformed boards explicit while returning readable matches", async () => {
		const test = harness({
			"board-1": "function invoice() {}",
			"board-broken": "function incomplete() {",
		});
		const response = await test.search();
		expect(hits(response)).toHaveLength(1);
		expect(response.coverage).toMatchObject({
			complete: false,
			issues: [
				expect.objectContaining({
					code: "RESOURCE_UNREADABLE",
					resource: "board-broken",
				}),
			],
		});
	});

	it.each([`invoice${"x".repeat(512)}`, `invoice${"界".repeat(470)}`])(
		"omits symbols whose exact references exceed character or encoded limits",
		async (name) => {
			const test = harness({
				"board-1": `function ${name}() {}\nfunction invoiceReadable() {}`,
			});
			const response = await test.search();
			const found = hits(response);
			expect(found).toHaveLength(1);
			expect(found[0].symbol).toBe("invoiceReadable");
			expect(parseWorkspaceResourceId(found[0].resource_id)).toBeDefined();
			expect(response.coverage).toMatchObject({
				complete: false,
				indexed_resources: 1,
				issues: [expect.objectContaining({ code: "RESOURCE_ID_INVALID" })],
			});
			expect(
				await readWorkspaceSymbol(test.backend, found[0], test.scope),
			).toMatchObject({ status: "ok" });
		},
	);

	it("ranks title and symbol evidence above the same words in a body", async () => {
		const documents = await Promise.all([
			document(
				{
					kind: "workflow",
					app_id: "app-1",
					board_id: "board-1",
					id: "invoice",
				},
				"invoice",
				"invoice",
				{ symbol: "invoice", signature: "function invoice()" },
			),
			document({ kind: "doc", id: "guide" }, "Reference", "invoice"),
		]);
		const ranked = rankWorkspaceDocuments(documents, "invoice");
		expect(ranked.hits.map((hit) => hit.resource_id)).toEqual(
			documents.map((entry) => entry.resource_id),
		);
		expect(ranked.hits[0].matched_fields).toEqual([
			"content",
			"signature",
			"symbol",
			"title",
		]);
		expect(ranked.hits[1].matched_fields).toEqual(["content"]);
	});

	it("returns reproducible ties and one hit per exact resource", async () => {
		const [a, b] = await Promise.all([
			document({ kind: "doc", id: "a" }, "Reference", "invoice"),
			document({ kind: "doc", id: "b" }, "Reference", "invoice"),
		]);
		const forward = rankWorkspaceDocuments([a, b, a], "invoice").hits;
		const reverse = rankWorkspaceDocuments([b, a, b], "invoice").hits;
		expect(forward).toEqual(reverse);
		expect(forward).toHaveLength(2);
		expect(forward.map((hit) => hit.resource_id)).toEqual([
			a.resource_id,
			b.resource_id,
		]);
	});

	it("reports fuzzy fallback and declines stop-word-only search terms", async () => {
		const entry = await document(
			{ kind: "doc", id: "guide" },
			"Accounting",
			"persistence",
		);
		expect(rankWorkspaceDocuments([entry], "persistnce")).toMatchObject({
			mode: "fuzzy",
			hits: [expect.objectContaining({ resource_id: entry.resource_id })],
		});
		expect(rankWorkspaceDocuments([entry], "find the existing")).toEqual({
			mode: "no_meaningful_terms",
			hits: [],
		});
	});

	it("searches and reads bounded bundled docs without consulting app access", async () => {
		const test = harness({});
		test.getProfileAppIds.mockRejectedValue(new Error("Profile offline."));
		const response = await test.search({ kinds: ["doc"] });
		const found = hits(response);
		expect(found).toHaveLength(1);
		expect(response.coverage).toMatchObject({
			complete: true,
			docs: { mode: "bundled_selected_docs" },
		});
		expect(found[0].matched_fields).toEqual(["content"]);
		expect(
			await readWorkspaceSymbol(test.backend, found[0], test.scope),
		).toMatchObject({
			status: "ok",
			source_path: "dev/workflows.mdx",
			start_line: 12,
			end_line: 14,
			content: "Reuse an invoice helper before creating a new workflow.",
		});
		expect(test.getProfileAppIds).not.toHaveBeenCalled();
		expect(test.getBoardSummariesAuthoritative).not.toHaveBeenCalled();
	});
});

describe("FlowPilot workspace pagination validity", () => {
	const source =
		"function invoiceA() {}\nfunction invoiceB() {}\nfunction invoiceC() {}";

	it("revalidates cached result content and refuses changed source", async () => {
		const test = harness({ "board-1": source });
		const first = await test.search({ limit: 1 });
		expect(hits(first)).toHaveLength(1);
		expect(first.next_cursor).toEqual(expect.any(String));
		test.sources["board-1"] = source.replace(
			"invoiceB() {}",
			'invoiceB() { return "changed" }',
		);
		const next = await test.search({ limit: 1, cursor: first.next_cursor });
		expect(next).toMatchObject({
			status: "stale",
			code: "WORKSPACE_SNAPSHOT_CHANGED",
		});
		expect(next).not.toHaveProperty("hits");
		expect(
			await test.search({ limit: 1, cursor: first.next_cursor }),
		).toMatchObject({ code: "WORKSPACE_CURSOR_INVALID" });
	});

	it("does not return cached snippets after profile access is revoked", async () => {
		const test = harness({ "board-1": source });
		const first = await test.search({ limit: 1 });
		test.getFlowScriptAuthoritative.mockClear();
		test.profile.clear();
		const next = await test.search({ limit: 1, cursor: first.next_cursor });
		expect(next).toMatchObject({ code: "WORKSPACE_CURSOR_INVALID" });
		expect(next).not.toHaveProperty("hits");
		expect(test.getFlowScriptAuthoritative).not.toHaveBeenCalled();
	});

	it("refuses cached content when the backend revokes access despite a retained profile", async () => {
		const test = harness({ "board-1": source });
		const first = await test.search({ limit: 1 });
		test.getFlowScriptAuthoritative.mockRejectedValue(
			new Error("Access denied."),
		);
		expect(await test.search({ cursor: first.next_cursor })).toMatchObject({
			status: "stale",
			code: "WORKSPACE_SNAPSHOT_CHANGED",
		});
	});

	it.each([
		{ query: "different" },
		{ board_id: "different-board" },
		{ kinds: ["workflow", "doc"] },
		{ app_id: "different-app" },
	])("rejects cursor reuse after changing query scope %j", async (change) => {
		const test = harness({ "board-1": source });
		const first = await test.search({ limit: 1 });
		expect(
			await test.search({ cursor: first.next_cursor, ...change }),
		).toMatchObject({ code: "WORKSPACE_CURSOR_INVALID" });
	});

	it("expires cursors and confines them to the originating search session", async () => {
		let now = 1_000;
		const test = harness({ "board-1": source }, { now: () => now });
		const first = await test.search({ limit: 1 });
		const other = new WorkspaceSearchSession(() => now);
		expect(
			await other.search(
				test.backend,
				{
					query: "invoice",
					app_id: "app-1",
					kinds: ["workflow"],
					cursor: first.next_cursor,
				},
				test.scope,
			),
		).toMatchObject({ code: "WORKSPACE_CURSOR_INVALID" });
		now += 120_001;
		expect(await test.search({ cursor: first.next_cursor })).toMatchObject({
			code: "WORKSPACE_CURSOR_INVALID",
		});
	});

	it("continues a valid search without duplicate hits or repeated inventory reads", async () => {
		const test = harness({ "board-1": source });
		const first = await test.search({ limit: 1 });
		const second = await test.search({ limit: 2, cursor: first.next_cursor });
		const combined = [...hits(first), ...hits(second)];
		expect(new Set(combined.map((hit) => hit.resource_id)).size).toBe(3);
		expect(second.next_cursor).toBeNull();
		expect(test.getBoardSummariesAuthoritative).toHaveBeenCalledTimes(1);
	});
});

describe("FlowPilot exact reference and response bounds", () => {
	it("pages Unicode and control characters by serialized byte size without losing content", async () => {
		const payload = "😀é\u0001\t".repeat(8_000);
		const source = `function invoice() { return "${payload}" }`;
		const test = harness({ "board-1": source });
		const hit = hits(await test.search())[0];
		let offset = 0;
		let reconstructed = "";
		let pages = 0;
		do {
			const read = await readWorkspaceSymbol(
				test.backend,
				{ ...hit, offset },
				test.scope,
			);
			expect(read.status).toBe("ok");
			const content = String(read.content);
			expect(jsonBytes(content)).toBeLessThanOrEqual(16_000);
			expect(jsonBytes(read)).toBeLessThanOrEqual(24_000);
			expect(content.length).toBeGreaterThan(0);
			expect(/[\uD800-\uDBFF]$/.test(content)).toBe(false);
			expect(/^[\uDC00-\uDFFF]/.test(content)).toBe(false);
			expect(read.offset).toBe(offset);
			reconstructed += content;
			pages++;
			if (read.next_offset === null) break;
			expect(read.next_offset).toBe(offset + content.length);
			offset = Number(read.next_offset);
			expect(pages).toBeLessThan(100);
		} while (offset < source.length);
		expect(pages).toBeGreaterThan(1);
		expect(reconstructed).toBe(source);
	});

	it("rejects offsets beyond the resource or inside a surrogate pair", async () => {
		const source = 'function invoice() { return "😀" }';
		const test = harness({ "board-1": source });
		const hit = hits(await test.search())[0];
		for (const offset of [source.length + 1, source.indexOf("😀") + 1]) {
			expect(
				await readWorkspaceSymbol(test.backend, { ...hit, offset }, test.scope),
			).toMatchObject({ code: "WORKSPACE_OFFSET_INVALID" });
		}
		for (const offset of [-1, 0.5, Number.MAX_SAFE_INTEGER + 1, "1"]) {
			test.getFlowScriptAuthoritative.mockClear();
			expect(
				await readWorkspaceSymbol(test.backend, { ...hit, offset }, test.scope),
			).toMatchObject({ code: "WORKSPACE_REFERENCE_INVALID" });
			expect(test.getFlowScriptAuthoritative).not.toHaveBeenCalled();
		}
	});

	it.each([
		undefined,
		"invoice",
		"workspace:v1:%zz",
		"workspace:v1:%5B%22workflow%22%2Cnull%2Cnull%2C%22invoice%22%5D",
		workspaceResourceId({ kind: "workflow", app_id: "app-1", id: "invoice" }),
		workspaceResourceId({ kind: "doc", app_id: "app-1", id: "guide" }),
		workspaceResourceId({
			kind: "workflow",
			app_id: "app-1",
			board_id: "board-1",
			id: "",
		}),
	])(
		"rejects malformed reference %s before backend calls",
		async (resource_id) => {
			const test = harness({ "board-1": "function invoice() {}" });
			expect(parseWorkspaceResourceId(resource_id)).toBeUndefined();
			expect(
				await readWorkspaceSymbol(
					test.backend,
					{ resource_id, revision: "a".repeat(64) },
					test.scope,
				),
			).toMatchObject({ code: "WORKSPACE_REFERENCE_INVALID" });
			expect(test.getProfileAppIds).not.toHaveBeenCalled();
			expect(test.getFlowScriptAuthoritative).not.toHaveBeenCalled();
		},
	);

	it("requires canonical resource IDs and exact hash-shaped revisions", async () => {
		const test = harness({ "board-1": "function invoice() {}" });
		const target: WorkspaceTarget = {
			kind: "workflow",
			app_id: "app-1",
			board_id: "board-1",
			id: "billing::invoice",
		};
		const id = workspaceResourceId(target);
		expect(parseWorkspaceResourceId(id)).toEqual(target);
		expect(
			parseWorkspaceResourceId(id.replaceAll("%3A", "%3a")),
		).toBeUndefined();
		for (const revision of [
			undefined,
			"latest",
			"a".repeat(63),
			"A".repeat(64),
		]) {
			expect(
				await readWorkspaceSymbol(
					test.backend,
					{ resource_id: id, revision },
					test.scope,
				),
			).toMatchObject({ code: "WORKSPACE_REFERENCE_INVALID" });
		}
		expect(test.getFlowScriptAuthoritative).not.toHaveBeenCalled();
	});

	it.each([
		{ query: "" },
		{ query: "x".repeat(257) },
		{ kinds: [] },
		{ kinds: ["secret"] },
		{ limit: 0 },
		{ limit: 21 },
		{ limit: 1.1 },
		{ app_id: undefined, board_id: "board-1" },
	])(
		"rejects invalid search arguments %j before backend calls",
		async (args) => {
			const test = harness({ "board-1": "function invoice() {}" });
			expect(await test.search(args)).toMatchObject({
				code: "WORKSPACE_QUERY_INVALID",
			});
			expect(test.getBoardSummariesAuthoritative).not.toHaveBeenCalled();
			expect(test.getFlowScriptAuthoritative).not.toHaveBeenCalled();
		},
	);

	it("bounds text by escaped JSON bytes rather than JavaScript character count", () => {
		const value = '😀\u0001"\\'.repeat(100);
		const bounded = boundedWorkspaceText(value, 100);
		expect(jsonBytes(bounded)).toBeLessThanOrEqual(100);
		expect(value.startsWith(bounded)).toBe(true);
		expect(/[\uD800-\uDBFF]$/.test(bounded)).toBe(false);
	});
});
