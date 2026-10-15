import { describe, expect, test } from "bun:test";
import type { IBoard } from "../schema/flow/board";
import { IPinType, IValueType, IVariableType } from "../schema/flow/board";
import type { INode } from "../schema/flow/node";
import {
	ROOT_SEGMENT,
	applyBoardSync,
	catalogByName,
	nodeSegment,
	toNode,
} from "./apply";
import { BoardSyncClient } from "./client";
import type {
	IBoardMeta,
	IBoardSyncManifest,
	IBoardSyncRequest,
	IBoardSyncResponse,
	ISyncNode,
} from "./types";

const manifest = (
	segments: Record<string, string>,
	rest: Partial<IBoardSyncManifest> = {},
): IBoardSyncManifest => ({
	meta: "m1",
	variables: "v1",
	comments: "c1",
	layers: { l1: "layer-l1" },
	segments,
	...rest,
});

const meta = (updated = 1): IBoardMeta => ({
	id: "b",
	name: "Board",
	description: "",
	viewport: [0, 0, 1],
	version: [0, 0, 1],
	stage: "Dev" as never,
	log_level: 1 as never,
	execution_mode: "Hybrid" as never,
	page_ids: [],
	hash: 7,
	created_at: { secs_since_epoch: 1, nanos_since_epoch: 0 },
	updated_at: { secs_since_epoch: updated, nanos_since_epoch: 0 },
});

const wireNode = (
	id: string,
	layer: string | null,
	extra: Partial<ISyncNode> = {},
): ISyncNode => ({
	id,
	name: "demo",
	version: 3,
	layer,
	friendly_name: "Demo",
	description: "d",
	category: "Test",
	pins: {
		[`${id}-p`]: {
			id: `${id}-p`,
			name: "value",
			index: 0,
			pin_type: IPinType.Input,
			data_type: IVariableType.String,
			value_type: IValueType.Normal,
			friendly_name: "Value",
			description: "",
			default_value: "ImhpIg==",
		},
	},
	...extra,
});

const catalogNode: INode = {
	id: "cat",
	name: "demo",
	version: 3,
	friendly_name: "Demo (catalog)",
	description: "from catalog",
	category: "Test",
	icon: "/i.svg",
	pins: {
		p: {
			id: "p",
			name: "value",
			index: 0,
			pin_type: IPinType.Input,
			data_type: IVariableType.String,
			value_type: IValueType.Normal,
			friendly_name: "Value (catalog)",
			description: "cat",
			connected_to: [],
			depends_on: [],
		},
	},
};

const layerDef = (id: string, name = id): IBoard["layers"][string] =>
	({
		id,
		name,
		type: "Collapsed",
		nodes: {},
		variables: {},
		comments: {},
		pins: {},
		coordinates: [0, 0, 0],
	}) as unknown as IBoard["layers"][string];

const fullResponse = (): IBoardSyncResponse => ({
	manifest: manifest({ [ROOT_SEGMENT]: "s-root", l1: "s-l1" }),
	meta: meta(),
	variables: {},
	comments: {},
	layers: { l1: layerDef("l1") },
	refs: { k: "{}" },
	segments: {
		[ROOT_SEGMENT]: { hash: "s-root", nodes: { a: wireNode("a", null) } },
		l1: { hash: "s-l1", nodes: { b: wireNode("b", "l1") } },
	},
});

describe("nodeSegment", () => {
	test("empty and missing layer are the root segment", () => {
		expect(nodeSegment({})).toBe(ROOT_SEGMENT);
		expect(nodeSegment({ layer: "" })).toBe(ROOT_SEGMENT);
		expect(nodeSegment({ layer: null })).toBe(ROOT_SEGMENT);
		expect(nodeSegment({ layer: "x" })).toBe("x");
	});
});

describe("applyBoardSync", () => {
	test("preserves per-instance auto reroute markers through catalog hydration", () => {
		const catalog = catalogByName([{ ...catalogNode, auto_reroute: true }]);
		for (const h of [false, true]) {
			for (const marker of [undefined, null, false, true]) {
				const { node, hydratable } = toNode(
					wireNode("a", null, { h, auto_reroute: marker }),
					catalog,
				);
				expect(hydratable).toBe(true);
				expect(node.auto_reroute).toBe(marker ?? null);
			}
		}
	});

	test("keeps board format requirements through full sync and partial updates", () => {
		const legacy = applyBoardSync(undefined, fullResponse(), undefined).board;
		expect(legacy.format_version).toBe(1);

		const upgraded = applyBoardSync(
			legacy,
			{
				meta: { ...meta(), format_version: 2 },
				manifest_delta: { meta: "m2" },
			},
			undefined,
		).board;
		expect(upgraded.format_version).toBe(2);

		const partial = applyBoardSync(
			upgraded,
			{ variables: {}, manifest_delta: { variables: "v2" } },
			undefined,
		).board;
		expect(partial.format_version).toBe(2);
		expect(
			applyBoardSync(partial, { manifest_delta: {} }, undefined).board,
		).toBe(partial);
	});

	test("assembles a full board from a full response", () => {
		const { board, changed } = applyBoardSync(
			undefined,
			fullResponse(),
			undefined,
		);
		expect(changed).toBe(true);
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "b"]);
		expect(board.refs).toEqual({ k: "{}" });
		expect(Object.keys(board.layers)).toEqual(["l1"]);
		expect(board.nodes.a.pins["a-p"].default_value).toEqual([
			0x22, 0x68, 0x69, 0x22,
		]);
		expect(board.nodes.a.pins["a-p"].connected_to).toEqual([]);
	});

	test("an empty diff returns the previous board by identity", () => {
		const first = applyBoardSync(undefined, fullResponse(), undefined).board;
		const { board, changed } = applyBoardSync(
			first,
			{ manifest: manifest({ [ROOT_SEGMENT]: "s-root", l1: "s-l1" }) },
			undefined,
		);
		expect(changed).toBe(false);
		expect(board).toBe(first);
	});

	test("a changed segment replaces its node set wholesale and leaves others by identity", () => {
		const first = applyBoardSync(undefined, fullResponse(), undefined).board;
		const { board } = applyBoardSync(
			first,
			{
				manifest: manifest(
					{ [ROOT_SEGMENT]: "s-root", l1: "s-l1-2" },
					{ meta: "m2" },
				),
				meta: meta(2),
				segments: {
					l1: { hash: "s-l1-2", nodes: { c: wireNode("c", "l1") } },
				},
			},
			undefined,
		);
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "c"]);
		expect(board.nodes.a).toBe(first.nodes.a);
		expect(board.updated_at.secs_since_epoch).toBe(2);
	});

	test("a node that moved layers is removed from the segment it left", () => {
		const first = applyBoardSync(undefined, fullResponse(), undefined).board;
		// `a` moved from root to l1: server resends l1 (grew) and drops root (now empty).
		const { board } = applyBoardSync(
			first,
			{
				manifest: manifest({ l1: "s-l1-3" }, { meta: "m3" }),
				meta: meta(3),
				segments: {
					l1: {
						hash: "s-l1-3",
						nodes: { a: wireNode("a", "l1"), b: wireNode("b", "l1") },
					},
				},
				dropped_segments: [ROOT_SEGMENT],
			},
			undefined,
		);
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "b"]);
		expect(board.nodes.a.layer).toBe("l1");
	});

	test("hydrates lean nodes from the catalog and flags nodes it cannot", () => {
		const catalog = catalogByName([catalogNode]);
		const lean = wireNode("a", null, {
			h: true,
			description: undefined,
			category: undefined,
			pins: {
				"a-p": { id: "a-p", name: "value", index: 0, default_value: null },
			},
		});
		const stale = wireNode("z", null, {
			h: true,
			version: 4,
			pins: { "z-p": { id: "z-p", name: "value", index: 0 } },
		});
		const { board, unhydratable } = applyBoardSync(
			undefined,
			{
				...fullResponse(),
				segments: {
					[ROOT_SEGMENT]: { hash: "h", nodes: { a: lean, z: stale } },
				},
			},
			catalog,
		);
		expect(board.nodes.a.friendly_name).toBe("Demo");
		expect(board.nodes.a.description).toBe("from catalog");
		expect(board.nodes.a.icon).toBe("/i.svg");
		expect(board.nodes.a.pins["a-p"].data_type).toBe(IVariableType.String);
		expect(board.nodes.a.pins["a-p"].friendly_name).toBe("Value (catalog)");
		expect(unhydratable.has(ROOT_SEGMENT)).toBe(true);
	});

	test("refs are upserted, never replaced", () => {
		const first = applyBoardSync(undefined, fullResponse(), undefined).board;
		const { board } = applyBoardSync(
			first,
			{
				manifest: manifest(
					{ [ROOT_SEGMENT]: "s-root-2", l1: "s-l1" },
					{ meta: "m2" },
				),
				meta: meta(2),
				refs: { k2: '{"new":true}' },
				segments: {
					[ROOT_SEGMENT]: {
						hash: "s-root-2",
						nodes: { a: wireNode("a", null) },
					},
				},
			},
			undefined,
		);
		expect(board.refs).toEqual({ k: "{}", k2: '{"new":true}' });
	});

	test("layer definitions merge by id and honour drops", () => {
		const first = applyBoardSync(undefined, fullResponse(), undefined).board;
		const { board } = applyBoardSync(
			first,
			{
				manifest: manifest(
					{ [ROOT_SEGMENT]: "s-root", l1: "s-l1" },
					{ meta: "m2", layers: { l1: "layer-l1", l2: "layer-l2" } },
				),
				meta: meta(2),
				layers: { l2: layerDef("l2", "Second") },
			},
			undefined,
		);
		expect(Object.keys(board.layers).sort()).toEqual(["l1", "l2"]);
		expect(board.layers.l1).toBe(first.layers.l1);

		const { board: after } = applyBoardSync(
			board,
			{
				manifest: manifest(
					{ [ROOT_SEGMENT]: "s-root", l1: "s-l1" },
					{ meta: "m3", layers: { l2: "layer-l2-renamed" } },
				),
				meta: meta(3),
				layers: { l2: layerDef("l2", "Renamed") },
				dropped_layers: ["l1"],
			},
			undefined,
		);
		expect(Object.keys(after.layers)).toEqual(["l2"]);
		expect(after.layers.l2.name).toBe("Renamed");
	});

	test("a full node never consults the catalog", () => {
		const catalog = catalogByName([catalogNode]);
		const { board, unhydratable } = applyBoardSync(
			undefined,
			fullResponse(),
			catalog,
		);
		expect(board.nodes.a.friendly_name).toBe("Demo");
		expect(unhydratable.size).toBe(0);
	});
});

describe("BoardSyncClient", () => {
	test("first sync sends an empty request, later syncs echo the manifest", async () => {
		const client = new BoardSyncClient();
		const requests: IBoardSyncRequest[] = [];
		const transport = async (request: IBoardSyncRequest) => {
			requests.push(request);
			return requests.length === 1
				? fullResponse()
				: { manifest: fullResponse().manifest };
		};
		const first = await client.sync("app", "b", undefined, transport);
		const second = await client.sync("app", "b", undefined, transport);
		expect(requests[0].segments).toBeUndefined();
		expect(requests[1].segments).toEqual({
			[ROOT_SEGMENT]: "s-root",
			l1: "s-l1",
		});
		expect(requests[1].meta).toBe("m1");
		expect(second).toBe(first);
	});

	test("retries stale-catalog segments without hydration", async () => {
		const client = new BoardSyncClient();
		client.setCatalog("app", [{ ...catalogNode, version: 2 }]);
		const requests: IBoardSyncRequest[] = [];
		const transport = async (request: IBoardSyncRequest) => {
			requests.push(request);
			if (requests.length === 1) {
				const response = fullResponse();
				const segments = response.segments ?? {};
				segments[ROOT_SEGMENT].nodes.a = wireNode("a", null, {
					h: true,
					description: undefined,
					pins: { "a-p": { id: "a-p", name: "value", index: 0 } },
				});
				return response;
			}
			expect(request.hydrate).toBe(false);
			expect(request.segments?.[ROOT_SEGMENT]).toBeUndefined();
			expect(request.segments?.l1).toBe("s-l1");
			return {
				manifest: fullResponse().manifest,
				segments: {
					[ROOT_SEGMENT]: { hash: "s-root", nodes: { a: wireNode("a", null) } },
				},
			};
		};
		const board = await client.sync("app", "b", undefined, transport);
		expect(requests).toHaveLength(2);
		expect(requests[0].hydrate).toBe(true);
		expect(board.nodes.a.description).toBe("d");
	});

	test("a sync that arrives mid-flight is answered by a request sent after it", async () => {
		const client = new BoardSyncClient();
		const requests: IBoardSyncRequest[] = [];
		const transport = async (request: IBoardSyncRequest) => {
			requests.push(request);
			return requests.length === 1
				? fullResponse()
				: { manifest: fullResponse().manifest };
		};
		const [a, b, c] = await Promise.all([
			client.sync("app", "b", undefined, transport),
			client.sync("app", "b", undefined, transport),
			client.sync("app", "b", undefined, transport),
		]);
		// One in-flight fetch plus one shared follow-up — never the stale in-flight answer.
		expect(requests).toHaveLength(2);
		expect(requests[1].segments).toEqual({
			[ROOT_SEGMENT]: "s-root",
			l1: "s-l1",
		});
		expect(a).toBe(b);
		expect(b).toBe(c);
	});

	test("ingest applies a merged-apply tail only onto the exact base it was computed against", async () => {
		const client = new BoardSyncClient();
		await client.sync("app", "b", undefined, async () => fullResponse());
		const base = client.syncRequest("app", "b", undefined);
		if (!base) throw new Error("a held board must yield a request");
		expect(base.patch).toBe(true);
		expect(base.segments).toEqual({ [ROOT_SEGMENT]: "s-root", l1: "s-l1" });

		const tail: IBoardSyncResponse = {
			manifest_delta: { meta: "m2", segments: { l1: "s-l1-2" } },
			meta: meta(2),
			segments: {
				l1: {
					hash: "s-l1-2",
					base: "s-l1",
					nodes: { c: wireNode("c", "l1") },
					removed: ["b"],
				},
			},
		};
		const board = client.ingest("app", "b", undefined, base, tail);
		if (!board) throw new Error("the tail matches the held base");
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "c"]);
		expect(client.peek("app", "b")).toBe(board);
		expect(client.syncRequest("app", "b", undefined)?.segments).toEqual({
			[ROOT_SEGMENT]: "s-root",
			l1: "s-l1-2",
		});
		expect(client.syncRequest("app", "b", undefined)?.meta).toBe("m2");

		// The same tail again: the held revision has moved on, so it is refused.
		expect(client.ingest("app", "b", undefined, base, tail)).toBeUndefined();
		expect(client.peek("app", "b")).toBe(board);
	});

	test("a plain sync whose response arrives after an ingest is discarded and re-asked", async () => {
		const client = new BoardSyncClient();
		await client.sync("app", "b", undefined, async () => fullResponse());
		const base = client.syncRequest("app", "b", undefined);
		if (!base) throw new Error("a held board must yield a request");

		const requests: IBoardSyncRequest[] = [];
		let release: () => void = () => {};
		const gate = new Promise<void>((resolve) => {
			release = resolve;
		});
		const transport = async (request: IBoardSyncRequest) => {
			requests.push(request);
			if (requests.length === 1) {
				await gate;
				// Computed against the old base: it must not roll the ingest back.
				return {
					manifest: fullResponse().manifest,
				} satisfies IBoardSyncResponse;
			}
			expect(request.meta).toBe("m2");
			return { manifest_delta: {} } satisfies IBoardSyncResponse;
		};
		const pending = client.sync("app", "b", undefined, transport);
		const ingested = client.ingest("app", "b", undefined, base, {
			manifest_delta: { meta: "m2" },
			meta: meta(2),
		});
		expect(ingested?.updated_at.secs_since_epoch).toBe(2);
		release();
		const board = await pending;
		expect(requests).toHaveLength(2);
		expect(board.updated_at.secs_since_epoch).toBe(2);
	});

	test("an ingest between response application and publication cannot be rolled back", async () => {
		const client = new BoardSyncClient();
		await client.sync("app", "b", undefined, async () => fullResponse());
		const base = client.syncRequest("app", "b")!;
		const published: IBoard[] = [];
		client.subscribe("app", "b", undefined, ({ board }) =>
			published.push(board),
		);
		let release!: (response: IBoardSyncResponse) => void;
		const response = new Promise<IBoardSyncResponse>((resolve) => {
			release = resolve;
		});
		const requests: IBoardSyncRequest[] = [];
		const pending = client.sync("app", "b", undefined, (request) => {
			requests.push(request);
			return requests.length === 1
				? response
				: Promise.resolve({ manifest_delta: {} });
		});
		release({ manifest_delta: {} });
		// The response resumes sync first; the command completes before sync publishes it.
		const committed = await new Promise<IBoard | undefined>((resolve) => {
			queueMicrotask(() => {
				resolve(
					client.ingest("app", "b", undefined, base, {
						manifest_delta: { meta: "m2" },
						meta: meta(2),
					}),
				);
			});
		});
		const result = await pending;
		expect(committed).toBeDefined();
		expect(result).toBe(committed!);
		expect(client.peek("app", "b")).toBe(committed!);
		expect(published).toEqual([committed!]);
		expect(requests).toHaveLength(2);
		expect(requests[1].meta).toBe("m2");
	});
});

describe("patches", () => {
	const held = () => applyBoardSync(undefined, fullResponse(), undefined).board;
	const request = (): IBoardSyncRequest => ({
		...manifest({ [ROOT_SEGMENT]: "s-root", l1: "s-l1" }),
		patch: true,
	});

	test("upserts and removals apply onto the held segment, untouched nodes keep identity", () => {
		const first = applyBoardSync(
			undefined,
			{
				...fullResponse(),
				segments: {
					[ROOT_SEGMENT]: {
						hash: "s-root",
						nodes: { a: wireNode("a", null), x: wireNode("x", null) },
					},
					l1: { hash: "s-l1", nodes: { b: wireNode("b", "l1") } },
				},
			},
			undefined,
		).board;
		const {
			board,
			manifest: next,
			unpatchable,
		} = applyBoardSync(
			first,
			{
				manifest_delta: {
					meta: "m2",
					segments: { [ROOT_SEGMENT]: "s-root-2" },
				},
				meta: meta(2),
				segments: {
					[ROOT_SEGMENT]: {
						hash: "s-root-2",
						base: "s-root",
						nodes: { y: wireNode("y", null) },
						removed: ["x"],
					},
				},
			},
			undefined,
			request(),
		);
		expect(unpatchable.size).toBe(0);
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "b", "y"]);
		expect(board.nodes.a).toBe(first.nodes.a);
		expect(board.nodes.b).toBe(first.nodes.b);
		expect(next).toEqual(
			manifest({ [ROOT_SEGMENT]: "s-root-2", l1: "s-l1" }, { meta: "m2" }),
		);
	});

	test("a patch onto a base the client did not send is refused, not applied", () => {
		const first = held();
		const { board, unpatchable } = applyBoardSync(
			first,
			{
				manifest_delta: { segments: { l1: "s-l1-2" } },
				segments: {
					l1: {
						hash: "s-l1-2",
						base: "some-other-revision",
						nodes: { c: wireNode("c", "l1") },
						removed: ["b"],
					},
				},
			},
			undefined,
			request(),
		);
		expect(unpatchable.has("l1")).toBe(true);
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "b"]);
	});

	test("the manifest is rebuilt from the request, the delta and the drops", () => {
		const { manifest: next } = applyBoardSync(
			held(),
			{
				manifest_delta: {
					variables: "v2",
					layers: { l2: "layer-l2" },
					segments: { l2: "s-l2" },
				},
				variables: {},
				layers: { l2: layerDef("l2") },
				dropped_layers: ["l1"],
				dropped_segments: ["l1"],
				segments: { l2: { hash: "s-l2", nodes: { q: wireNode("q", "l2") } } },
			},
			undefined,
			request(),
		);
		expect(next).toEqual({
			meta: "m1",
			variables: "v2",
			comments: "c1",
			layers: { l2: "layer-l2" },
			segments: { [ROOT_SEGMENT]: "s-root", l2: "s-l2" },
		});
	});

	test("the client re-requests refused patches whole", async () => {
		const client = new BoardSyncClient();
		await client.sync("app", "b", undefined, async () => fullResponse());
		const requests: IBoardSyncRequest[] = [];
		const transport = async (request: IBoardSyncRequest) => {
			requests.push(request);
			if (requests.length === 1) {
				return {
					manifest_delta: { segments: { l1: "s-l1-2" } },
					segments: {
						l1: {
							hash: "s-l1-2",
							base: "not-what-you-sent",
							nodes: { c: wireNode("c", "l1") },
						},
					},
				} satisfies IBoardSyncResponse;
			}
			expect(request.segments?.l1).toBeUndefined();
			expect(request.segments?.[ROOT_SEGMENT]).toBe("s-root");
			return {
				manifest_delta: { segments: { l1: "s-l1-2" } },
				segments: {
					l1: { hash: "s-l1-2", nodes: { c: wireNode("c", "l1") } },
				},
			} satisfies IBoardSyncResponse;
		};
		const board = await client.sync("app", "b", undefined, transport);
		expect(requests).toHaveLength(2);
		expect(Object.keys(board.nodes).sort()).toEqual(["a", "c"]);
	});
});

// Type-level guard: the assembled board is assignable to IBoard.
const _assign: IBoard = applyBoardSync(
	undefined,
	fullResponse(),
	undefined,
).board;
void _assign;

describe("hostile ids", () => {
	test("an id of __proto__ becomes an own entry instead of the map's prototype", () => {
		const response = fullResponse();
		const hostile = JSON.parse('{"__proto__": null}') as Record<
			string,
			ISyncNode
		>;
		hostile.__proto__ = wireNode("__proto__", null);
		response.segments = {
			[ROOT_SEGMENT]: { hash: "s-root", nodes: hostile },
			l1: { hash: "s-l1", nodes: { b: wireNode("b", "l1") } },
		};
		response.layers = JSON.parse(
			`{"__proto__": ${JSON.stringify(layerDef("__proto__"))}}`,
		);
		const { board } = applyBoardSync(undefined, response, undefined);

		expect(Object.getPrototypeOf(board.nodes)).toBe(Object.prototype);
		expect(Object.keys(board.nodes).sort()).toEqual(["__proto__", "b"]);
		expect(Object.getPrototypeOf(board.layers)).toBe(Object.prototype);
		expect(Object.keys(board.layers)).toEqual(["__proto__"]);
		expect(Object.keys(board.nodes.__proto__?.pins ?? {})).toEqual([
			"__proto__-p",
		]);
	});
});

describe("incremental graph identity", () => {
	test("metadata and refs do not rebuild graph maps", () => {
		const initial = applyBoardSync(undefined, fullResponse(), undefined);
		const updated = applyBoardSync(
			initial.board,
			{ meta: meta(2), refs: { other: "{}" } },
			undefined,
		);
		expect(updated.board.nodes).toBe(initial.board.nodes);
		expect(updated.board.layers).toBe(initial.board.layers);
		expect(updated.board.comments).toBe(initial.board.comments);
		expect(updated.changes).toEqual({
			meta: true,
			variables: false,
			comments: false,
			refs: ["other"],
			layers: { upserted: [], removed: [] },
			nodes: { upserted: [], removed: [] },
		});
	});

	test("refs-only responses apply, repeated refs preserve the snapshot", () => {
		const initial = applyBoardSync(undefined, fullResponse(), undefined);
		const updated = applyBoardSync(
			initial.board,
			{ refs: { k: "new" } },
			undefined,
		);
		expect(updated.board.refs.k).toBe("new");
		expect(initial.board.refs.k).toBe("{}");
		const repeated = applyBoardSync(
			updated.board,
			{ refs: { k: "new" } },
			undefined,
		);
		expect(repeated.changed).toBe(false);
		expect(repeated.board).toBe(updated.board);
	});

	test("patches retain unrelated identities and keep the segment index current after moves", () => {
		const initial = applyBoardSync(undefined, fullResponse(), undefined);
		const moved = applyBoardSync(
			initial.board,
			{
				segments: {
					l1: { hash: "l2", base: "s-l1", nodes: { a: wireNode("a", "l1") } },
					[ROOT_SEGMENT]: {
						hash: "r2",
						base: "s-root",
						removed: ["a"],
						nodes: {},
					},
				},
			},
			undefined,
			initial.manifest,
		);
		expect(moved.board.nodes.b).toBe(initial.board.nodes.b);
		expect(moved.board.layers).toBe(initial.board.layers);
		expect(moved.board.refs).toBe(initial.board.refs);
		expect(moved.changes.nodes).toEqual({ upserted: ["a"], removed: [] });
		const removed = applyBoardSync(
			moved.board,
			{ dropped_segments: ["l1"] },
			undefined,
			moved.manifest,
		);
		expect(Object.keys(removed.board.nodes)).toEqual([]);
		expect(removed.changes.nodes.removed.sort()).toEqual(["a", "b"]);
		expect(initial.board.nodes.a.layer).toBe(null);
	});

	test("a rejected patch has no graph changes", () => {
		const initial = applyBoardSync(undefined, fullResponse(), undefined);
		const rejected = applyBoardSync(
			initial.board,
			{
				segments: {
					[ROOT_SEGMENT]: {
						hash: "new",
						base: "wrong",
						nodes: {},
						removed: ["a"],
					},
				},
			},
			undefined,
			initial.manifest,
		);
		expect(rejected.board).toBe(initial.board);
		expect(rejected.changes.nodes).toEqual({ upserted: [], removed: [] });
		expect(rejected.unpatchable.has(ROOT_SEGMENT)).toBe(true);
	});

	test("board scopes and forgetting distinguish overlapping app and board IDs", async () => {
		const client = new BoardSyncClient();
		const first = await client.sync("ab", "c", undefined, async () =>
			fullResponse(),
		);
		const second = await client.sync("a", "bc", undefined, async () =>
			fullResponse(),
		);
		const third = await client.sync("ab", "c-more", undefined, async () =>
			fullResponse(),
		);
		expect(client.peek("ab", "c")).toBe(first);
		expect(client.peek("a", "bc")).toBe(second);
		client.forget("ab", "c");
		expect(client.peek("ab", "c")).toBeUndefined();
		expect(client.peek("a", "bc")).toBe(second);
		expect(client.peek("ab", "c-more")).toBe(third);
		client.forget(undefined, "bc");
		expect(client.peek("a", "bc")).toBeUndefined();
		expect(client.peek("ab", "c-more")).toBe(third);
	});
});

describe("board change subscriptions", () => {
	test("publishes sync and merged edits only to the matching scope", async () => {
		const client = new BoardSyncClient();
		const updates: import("./client").BoardSyncUpdate[] = [];
		const unsubscribe = client.subscribe("app", "board", undefined, (update) =>
			updates.push(update),
		);
		await client.sync("other", "board", undefined, async () => fullResponse());
		expect(updates).toHaveLength(0);
		const first = await client.sync("app", "board", undefined, async () =>
			fullResponse(),
		);
		expect(updates).toHaveLength(1);
		expect(updates[0].previous).toBeUndefined();
		expect(updates[0].board).toBe(first);
		await client.sync("app", "board", undefined, async () => ({
			manifest: fullResponse().manifest,
		}));
		expect(updates).toHaveLength(1);
		const updated = client.ingest(
			"app",
			"board",
			undefined,
			client.syncRequest("app", "board")!,
			{ refs: { new: "ref" }, manifest_delta: {} },
		);
		expect(updates).toHaveLength(2);
		expect(updates[1].previous).toBe(first);
		expect(updated).toBeDefined();
		expect(updates[1].board).toBe(updated!);
		expect(updates[1].changes.refs).toEqual(["new"]);
		unsubscribe();
		await client.sync("app", "board", undefined, async () => ({
			meta: meta(2),
		}));
		expect(updates).toHaveLength(2);
	});

	test("combines hydration retry changes and publishes only the complete board", async () => {
		const client = new BoardSyncClient();
		const updates: import("./client").BoardSyncUpdate[] = [];
		client.subscribe("app", "board", undefined, (update) =>
			updates.push(update),
		);
		const source = fullResponse();
		source.segments![ROOT_SEGMENT].nodes.a.h = true;
		let requests = 0;
		const board = await client.sync("app", "board", undefined, async () => {
			if (++requests === 1) return source;
			expect(updates).toHaveLength(0);
			return {
				segments: {
					[ROOT_SEGMENT]: { hash: "full", nodes: { a: wireNode("a", null) } },
				},
			};
		});
		expect(requests).toBe(2);
		expect(updates).toHaveLength(1);
		expect(updates[0].board).toBe(board);
		expect(updates[0].changes.nodes.upserted.sort()).toEqual(["a", "b"]);
		expect(updates[0].changes.layers.upserted).toEqual(["l1"]);
		expect(updates[0].changes.meta).toBe(true);
	});

	test("a failed hydration retry retains the last complete revision", async () => {
		const client = new BoardSyncClient();
		const first = await client.sync("app", "board", undefined, async () =>
			fullResponse(),
		);
		const updates: import("./client").BoardSyncUpdate[] = [];
		client.subscribe("app", "board", undefined, (update) =>
			updates.push(update),
		);
		let requests = 0;
		await expect(
			client.sync("app", "board", undefined, async () => {
				if (++requests === 1)
					return {
						segments: {
							[ROOT_SEGMENT]: {
								hash: "lean",
								nodes: { a: wireNode("a", null, { h: true }) },
							},
						},
					};
				throw new Error("offline");
			}),
		).rejects.toThrow("offline");
		expect(updates).toHaveLength(0);
		expect(client.peek("app", "board")).toBe(first);
	});
});

test("mixed incremental segment edits match full reconstruction without changing older snapshots", () => {
	const segments = [ROOT_SEGMENT, "l1", "l2"];
	let authoritative: Record<string, ISyncNode> = Object.fromEntries(
		Array.from({ length: 150 }, (_, i) => [
			`n${i}`,
			wireNode(`n${i}`, i % 3 === 0 ? null : `l${i % 3}`),
		]),
	);
	let revision = 1;
	const full = (): IBoardSyncResponse => ({
		...fullResponse(),
		manifest: manifest(
			Object.fromEntries(segments.map((id) => [id, `s${revision}-${id}`])),
		),
		segments: Object.fromEntries(
			segments.map((id) => [
				id,
				{
					hash: `s${revision}-${id}`,
					nodes: Object.fromEntries(
						Object.entries(authoritative).filter(
							([, node]) => nodeSegment(node) === id,
						),
					),
				},
			]),
		),
	});
	let incremental = applyBoardSync(undefined, full(), undefined);
	let seed = 117;
	const next = (bound: number) => {
		seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
		return seed % bound;
	};
	for (let step = 0; step < 90; step++) {
		const before = incremental.board;
		const saved = JSON.stringify(before.nodes);
		const old = authoritative;
		authoritative = { ...old };
		const id = `n${next(180)}`;
		const segment = segments[next(segments.length)];
		if (step % 5 === 0) delete authoritative[id];
		else
			authoritative[id] = wireNode(
				id,
				segment === ROOT_SEGMENT ? null : segment,
				{ coordinates: [step, -step, 0] },
			);
		revision++;
		const target = full();
		const patchSegments: NonNullable<IBoardSyncResponse["segments"]> = {};
		for (const segment of segments) {
			if (step % 9 === 0) {
				patchSegments[segment] = target.segments![segment];
				continue;
			}
			const nodes = Object.fromEntries(
				Object.entries(authoritative).filter(
					([id, node]) => nodeSegment(node) === segment && old[id] !== node,
				),
			);
			const removed = Object.entries(old)
				.filter(
					([id, node]) =>
						nodeSegment(node) === segment &&
						(!authoritative[id] || nodeSegment(authoritative[id]) !== segment),
				)
				.map(([id]) => id);
			patchSegments[segment] = {
				hash: target.segments![segment].hash,
				base: incremental.manifest.segments[segment],
				nodes,
				removed,
			};
		}
		incremental = applyBoardSync(
			before,
			{ segments: patchSegments, manifest: target.manifest },
			undefined,
			incremental.manifest,
		);
		const rebuilt = applyBoardSync(undefined, target, undefined);
		expect(incremental.board.nodes).toEqual(rebuilt.board.nodes);
		expect(JSON.stringify(before.nodes)).toBe(saved);
		const expectedRemoved = Object.keys(before.nodes).filter(
			(id) => !incremental.board.nodes[id],
		);
		expect(incremental.changes.nodes.removed.sort()).toEqual(
			expectedRemoved.sort(),
		);
	}
});

test("commands arriving during hydration cannot publish or obtain an incomplete revision", async () => {
	const client = new BoardSyncClient();
	const first = await client.sync("app", "board", undefined, async () =>
		fullResponse(),
	);
	const updates: import("./client").BoardSyncUpdate[] = [];
	client.subscribe("app", "board", undefined, (update) => updates.push(update));
	let finishRetry!: (response: IBoardSyncResponse) => void;
	let enteredRetry!: () => void;
	const retryStarted = new Promise<void>((resolve) => {
		enteredRetry = resolve;
	});
	let requests = 0;
	const partialManifest = manifest({ [ROOT_SEGMENT]: "lean", l1: "s-l1" });
	const pending = client.sync("app", "board", undefined, async () => {
		if (++requests === 1)
			return {
				manifest: partialManifest,
				segments: {
					[ROOT_SEGMENT]: {
						hash: "lean",
						nodes: {
							a: wireNode("a", null, { h: true, description: undefined }),
						},
					},
				},
			};
		enteredRetry();
		return new Promise((resolve) => {
			finishRetry = resolve;
		});
	});
	await retryStarted;
	expect(client.peek("app", "board")).toBe(first);
	expect(client.syncRequest("app", "board")).toBeUndefined();
	expect(
		client.ingest("app", "board", undefined, partialManifest, {
			meta: meta(3),
			manifest: partialManifest,
		}),
	).toBeUndefined();
	expect(updates).toHaveLength(0);
	finishRetry({
		segments: {
			[ROOT_SEGMENT]: { hash: "complete", nodes: { a: wireNode("a", null) } },
		},
		manifest_delta: { segments: { [ROOT_SEGMENT]: "complete" } },
	});
	const complete = await pending;
	expect(complete.nodes.a.description).toBe("d");
	expect(client.peek("app", "board")).toBe(complete);
	expect(updates).toHaveLength(1);
	expect(updates[0].board).toBe(complete);
});
