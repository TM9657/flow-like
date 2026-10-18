import { describe, expect, spyOn, test } from "bun:test";
import { type FlowSceneEdge, edgeNeedsDom } from "./board-canvas/scene";
import { doPinsMatch, isValidConnection, parseBoard } from "./flow-board-utils";
import { GEOMETRY_KINDS, geometryMarker } from "./geometry";
import type {
	IBoard,
	IComment,
	ILayer,
	ILayerCache,
	IVariable,
} from "./schema/flow/board";
import {
	ICommentType,
	IExecutionMode,
	ILayerCacheScope,
	ILayerType,
} from "./schema/flow/board";
import type { INode } from "./schema/flow/node";
import { IVariableType } from "./schema/flow/node";
import type { IPin } from "./schema/flow/pin";
import { IPinType, IValueType } from "./schema/flow/pin";
import { convertJsonToUint8Array, parseUint8ArrayToJson } from "./uint8";

/** Only the slice of a rendered React Flow node these tests read. */
interface IRenderedNode {
	id: string;
	position?: { x: number; y: number };
	data: {
		node: INode;
		executionMode?: IExecutionMode;
		isOffline?: boolean;
		onRemoteExecute?: (node: INode, payload?: object) => Promise<void>;
		comment?: IComment;
		presignedUrl?: string;
		boardDataVersion?: string;
		boardContentVersion?: string;
		functionCache?: ILayerCache;
	};
}

const variable = (id: string, name: string): IVariable =>
	({
		id,
		name,
		category: null,
		data_type: IVariableType.String,
		value_type: IValueType.Normal,
		exposed: false,
		secret: false,
		editable: true,
	}) as IVariable;

const getNode = (id: string, varRef: string): INode =>
	({
		id,
		name: "variable_get",
		friendly_name: `Get ${varRef}`,
		description: "",
		category: "Variables",
		coordinates: [0, 0, 0],
		hash: 1,
		pins: {
			p1: {
				id: "p1",
				name: "var_ref",
				friendly_name: "Variable",
				description: "",
				pin_type: "Input",
				data_type: IVariableType.String,
				value_type: IValueType.Normal,
				index: 0,
				connected_to: [],
				depends_on: [],
				default_value: convertJsonToUint8Array(varRef),
			},
		},
	}) as unknown as INode;

const functionLayer = (id: string, overrides: Partial<ILayer> = {}): ILayer =>
	({
		id,
		name: "fn",
		type: ILayerType.Function,
		nodes: {},
		pins: {},
		variables: {},
		comments: {},
		coordinates: [0, 0, 0],
		parent_id: null,
		...overrides,
	}) as unknown as ILayer;

const board = (overrides: Partial<IBoard> = {}): IBoard =>
	({
		id: "board-1",
		name: "b",
		nodes: { n1: getNode("n1", "v1") },
		variables: { v1: variable("v1", "Counter") },
		layers: { l1: functionLayer("l1") },
		comments: {},
		refs: {},
		version: [0, 0, 1],
		...overrides,
	}) as unknown as IBoard;

interface IParsed {
	nodes: IRenderedNode[];
	edges: unknown[];
}

const parse = (
	source: IBoard,
	oldNodes?: IRenderedNode[],
	oldEdges?: unknown[],
): IParsed =>
	parseBoard(
		source,
		"app-1",
		() => {},
		() => {},
		async () => {},
		async () => undefined,
		new Set<string>(),
		undefined,
		oldNodes,
		oldEdges,
		undefined,
		{ current: source },
	) as IParsed;

const node = (parsed: IParsed, id: string): IRenderedNode => {
	const found = parsed.nodes.find((candidate) => candidate.id === id);
	if (!found) throw new Error(`node ${id} was not rendered`);
	return found;
};

describe("historical canvas controls", () => {
	test("cached nodes refresh their read-only version even when the board object is unchanged", () => {
		const source = board();
		const latest = parse(source);
		const historical = parseBoard(
			source,
			"app-1",
			() => {},
			() => {},
			async () => {},
			async () => undefined,
			new Set<string>(),
			undefined,
			latest.nodes,
			latest.edges,
			undefined,
			{ current: source },
			[0, 0, 1],
		);
		expect(
			historical.nodes.find((node) => node.id === "n1")?.data.version,
		).toEqual([0, 0, 1]);
		const restored = parseBoard(
			source,
			"app-1",
			() => {},
			() => {},
			async () => {},
			async () => undefined,
			new Set<string>(),
			undefined,
			historical.nodes,
			historical.edges,
			undefined,
			{ current: source },
		);
		expect(
			restored.nodes.find((node) => node.id === "n1")?.data.version,
		).toBeUndefined();
	});

	test("historical layers and boundary pins carry version and cannot submit mutations", async () => {
		const layer = functionLayer("layer", { type: ILayerType.Collapsed });
		const source = board({ layers: { layer } });
		const commands: unknown[] = [];
		for (const currentLayer of [undefined, "layer"]) {
			const parsed = parseBoard(
				source,
				"app-1",
				() => {},
				() => {},
				async () => {},
				async (command) => {
					commands.push(command);
				},
				new Set<string>(),
				undefined,
				undefined,
				undefined,
				currentLayer,
				{ current: source },
				[0, 0, 1],
			);
			const layers = parsed.nodes.filter((node) => node.data.layer);
			expect(layers.length).toBe(currentLayer ? 2 : 1);
			for (const node of layers) {
				expect(node.data.version).toEqual([0, 0, 1]);
				await node.data.onLayerUpdate({ ...layer, name: "Edited" });
				await node.data.onLayerRemove(layer, false);
			}
		}
		expect(commands).toEqual([]);
	});
});

test("optional layer boundary coordinates use runtime defaults before their first move", () => {
	const source = board({ layers: { layer: functionLayer("layer") } });
	const parsed = parseBoard(
		source,
		"app-1",
		() => {},
		() => {},
		async () => {},
		async () => undefined,
		new Set<string>(),
		undefined,
		undefined,
		undefined,
		"layer",
		{ current: source },
	);
	expect(
		parsed.nodes.find((node) => node.id === "layer-input")?.position,
	).toEqual({ x: -200, y: 0 });
	expect(
		parsed.nodes.find((node) => node.id === "layer-return")?.position,
	).toEqual({ x: 200, y: 0 });
	expect(source.layers.layer.in_coordinates).toBeUndefined();
	expect(source.layers.layer.out_coordinates).toBeUndefined();
});

test("cached function-reference edges follow path preference and selection changes", () => {
	const from = getNode("from", "v1");
	const to = getNode("to", "v1");
	from.fn_refs = {
		can_reference_fns: true,
		can_be_referenced_by_fns: false,
		fn_refs: ["to"],
	};
	to.fn_refs = {
		can_reference_fns: false,
		can_be_referenced_by_fns: true,
		fn_refs: [],
	};
	const source = board({ nodes: { from, to }, layers: {} });
	const initial = parseBoard(
		source,
		"app-1",
		() => {},
		() => {},
		async () => {},
		async () => undefined,
		new Set<string>(),
		"default",
	);
	const id = "ref_out_from-ref_in_to";
	const changed = parseBoard(
		source,
		"app-1",
		() => {},
		() => {},
		async () => {},
		async () => undefined,
		new Set([id]),
		"straight",
		initial.nodes,
		initial.edges,
	);
	expect(changed.edges.find((edge) => edge.id === id)).toMatchObject({
		selected: true,
		data: { pathType: "straight" },
	});
	const cleared = parseBoard(
		source,
		"app-1",
		() => {},
		() => {},
		async () => {},
		async () => undefined,
		new Set<string>(),
		"straight",
		changed.nodes,
		changed.edges,
	);
	expect(cleared.edges.find((edge) => edge.id === id)?.selected).toBe(false);
});

const contentVersion = (parsed: IParsed, id: string) =>
	node(parsed, id).data.boardContentVersion;

const callNode = (id: string, layerId: string): INode =>
	({
		id,
		name: "control_call_function",
		friendly_name: "Call fn",
		description: "",
		category: "Control",
		coordinates: [0, 0, 0],
		hash: 2,
		pins: {
			p1: {
				id: "p1",
				name: "function_layer_id",
				friendly_name: "Function",
				description: "",
				pin_type: "Input",
				data_type: IVariableType.String,
				value_type: IValueType.Normal,
				index: 0,
				connected_to: [],
				depends_on: [],
				default_value: convertJsonToUint8Array(layerId),
			},
		},
	}) as unknown as INode;

/**
 * The version tokens are identity-based, relying on react-query's structural
 * sharing to keep unchanged sub-objects referentially stable across refetches.
 * Spreading the previous board reproduces that: every slice keeps its reference
 * except the one the test actually changes.
 */
const edit = (previous: IBoard, change: Partial<IBoard>): IBoard => ({
	...previous,
	...change,
});

describe("media comment URLs", () => {
	for (const commentType of [ICommentType.Image, ICommentType.Video]) {
		const mediaBoard = (presignedUrl?: string) =>
			board({
				comments: {
					media: {
						id: "media",
						comment_type: commentType,
						content: "pasted-media.png",
						coordinates: [100, 200, 0],
						timestamp: { secs_since_epoch: 1, nanos_since_epoch: 0 },
						hash: 42,
						presigned_url: presignedUrl,
					},
				},
			});

		test.each([
			["arrives after sync", undefined, "https://storage.example/media?sig=1"],
			[
				"is refreshed",
				"https://storage.example/media?sig=1",
				"https://storage.example/media?sig=2",
			],
			["is removed", "https://storage.example/media?sig=1", undefined],
		])(
			`${commentType} updates when its URL %s without moving`,
			(_, before, after) => {
				const first = parse(mediaBoard(before));
				const second = parse(mediaBoard(after), first.nodes, first.edges);
				const rendered = node(second, "media");

				expect(rendered).not.toBe(node(first, "media"));
				expect(rendered.data.presignedUrl).toBe(after);
				expect(rendered.data.comment?.presigned_url).toBe(after);
				expect(rendered.data.comment?.coordinates).toEqual([100, 200, 0]);
			},
		);

		test(`${commentType} keeps its rendered node when its URL is unchanged`, () => {
			const source = mediaBoard("https://storage.example/media?sig=1");
			const first = parse(source);
			const second = parse(source, first.nodes, first.edges);
			expect(node(second, "media")).toBe(node(first, "media"));
		});

		test(`${commentType} keeps an active drag and resize when its URL arrives`, () => {
			const first = parse(mediaBoard());
			const interactionState = {
				position: { x: 150, y: 250 },
				dragging: true,
				selected: true,
				width: 500,
				height: 350,
				measured: { width: 500, height: 350 },
			};
			const previous = { ...node(first, "media"), ...interactionState };
			const second = parse(
				mediaBoard("https://storage.example/media?sig=1"),
				[previous],
				first.edges,
			);

			expect(node(second, "media")).toMatchObject(interactionState);
			expect(node(second, "media").data.presignedUrl).toBe(
				"https://storage.example/media?sig=1",
			);
		});
	}
});

describe("boardContentVersion on rendered nodes", () => {
	test("changes when a variable is renamed, so variable nodes re-render", () => {
		const before = board();
		const first = parse(before);
		const token = contentVersion(first, "n1");
		expect(token).toBeString();

		// A rename replaces the variables map. The node itself is untouched, so its
		// backend hash — the memo comparator's primary key — does not move, and the
		// content version is the only signal that its pins must re-resolve.
		const second = parse(
			edit(before, { variables: { v1: variable("v1", "Renamed") } }),
			first.nodes,
			first.edges,
		);

		expect(node(second, "n1").data.node.hash).toBe(
			node(first, "n1").data.node.hash,
		);
		expect(contentVersion(second, "n1")).not.toBe(token);
	});

	test("changes when a function layer is renamed", () => {
		const before = board();
		const first = parse(before);
		const second = parse(
			edit(before, {
				layers: { l1: functionLayer("l1", { name: "renamed" }) },
			}),
			first.nodes,
			first.edges,
		);
		expect(contentVersion(second, "n1")).not.toBe(contentVersion(first, "n1"));
	});

	test("changes when a LOCAL variable is renamed", () => {
		// Local variables live on the function layer, and a var_ref pin lists them
		// alongside the board's own — a rename there must reach the node too.
		const before = board({
			layers: {
				l1: functionLayer("l1", { variables: { lv: variable("lv", "Local") } }),
			},
		});
		const first = parse(before);
		const second = parse(
			edit(before, {
				layers: {
					l1: functionLayer("l1", {
						variables: { lv: variable("lv", "LocalRenamed") },
					}),
				},
			}),
			first.nodes,
			first.edges,
		);
		expect(contentVersion(second, "n1")).not.toBe(contentVersion(first, "n1"));
	});

	test("changes when a local variable MOVES to another layer", () => {
		// The dropdown a var_ref pin renders lists the current layer's locals, so a
		// variable changing owner changes what a mounted node offers.
		const before = board({
			layers: {
				l1: functionLayer("l1", { variables: { lv: variable("lv", "Local") } }),
				l2: functionLayer("l2", { name: "other" }),
			},
		});
		const first = parse(before);
		const second = parse(
			edit(before, {
				layers: {
					l1: functionLayer("l1"),
					l2: functionLayer("l2", {
						name: "other",
						variables: { lv: variable("lv", "Local") },
					}),
				},
			}),
			first.nodes,
			first.edges,
		);
		expect(contentVersion(second, "n1")).not.toBe(contentVersion(first, "n1"));
	});

	test("changes when a local variable is RETYPED", () => {
		const before = board({
			layers: {
				l1: functionLayer("l1", { variables: { lv: variable("lv", "Local") } }),
			},
		});
		const first = parse(before);
		const second = parse(
			edit(before, {
				layers: {
					l1: functionLayer("l1", {
						variables: {
							lv: {
								...variable("lv", "Local"),
								data_type: IVariableType.Integer,
							},
						},
					}),
				},
			}),
			first.nodes,
			first.edges,
		);
		expect(contentVersion(second, "n1")).not.toBe(contentVersion(first, "n1"));
	});

	test("does not change when a layer is DRAGGED", () => {
		// The whole point of the signature: coordinates move constantly and no node
		// renders them, so a layer drag must not repaint the canvas.
		const before = board();
		const first = parse(before);
		const token = contentVersion(first, "n1");

		const second = parse(
			edit(before, {
				layers: { l1: functionLayer("l1", { coordinates: [640, 480, 0] }) },
			}),
			first.nodes,
			first.edges,
		);

		expect(node(second, "n1").data.boardDataVersion).not.toBe(
			node(first, "n1").data.boardDataVersion,
		);
		expect(contentVersion(second, "n1")).toBe(token);
	});

	test("refreshes functionCache when the called function toggles caching", () => {
		// A call node's own hash does not move when the function it points at
		// changes its caching, so every rebuild path has to recompute the badge.
		const before = board({
			nodes: { c1: callNode("c1", "l1") },
			layers: { l1: functionLayer("l1") },
		});
		const first = parse(before);
		expect(node(first, "c1").data.functionCache).toBeUndefined();

		const second = parse(
			edit(before, {
				layers: {
					l1: functionLayer("l1", {
						cache: {
							enabled: true,
							prefix: "p",
							ttl_seconds: 60,
							scope: ILayerCacheScope.App,
						},
					}),
				},
			}),
			first.nodes,
			first.edges,
		);

		// Same node hash — this is the shallow-update path, not a full rebuild.
		expect(node(second, "c1").data.node.hash).toBe(
			node(first, "c1").data.node.hash,
		);
		expect(node(second, "c1").data.functionCache?.enabled).toBe(true);
		expect(node(second, "c1").data.functionCache?.ttl_seconds).toBe(60);
	});

	test("keeps the function_layer_id pin on the rendered call node", () => {
		// The hover toolbar's Edit action resolves the function through this pin. It is
		// hidden from the pin rows, never removed from the node the renderer carries.
		const before = board({
			nodes: { c1: callNode("c1", "l1") },
			layers: { l1: functionLayer("l1") },
		});
		const parsed = parse(before);
		const pin = Object.values(node(parsed, "c1").data.node.pins).find(
			(candidate) => candidate.name === "function_layer_id",
		);
		expect(parseUint8ArrayToJson(pin?.default_value)).toBe("l1");
	});

	test("does not change when board.refs churns", () => {
		// refs gain a key whenever a node type appears for the first time. No mounted
		// node reads them — the editors that do are dialogs that snapshot on open.
		const before = board();
		const first = parse(before);
		const second = parse(
			edit(before, { refs: { abc: "{}" } }),
			first.nodes,
			first.edges,
		);
		expect(contentVersion(second, "n1")).toBe(contentVersion(first, "n1"));
	});

	test("does not change when an unrelated node is added", () => {
		const before = board();
		const first = parse(before);
		const token = contentVersion(first, "n1");

		const second = parse(
			edit(before, { nodes: { ...before.nodes, n2: getNode("n2", "v1") } }),
			first.nodes,
			first.edges,
		);

		// Membership moved, so parseBoard's own reuse key must change...
		expect(node(second, "n1").data.boardDataVersion).not.toBe(
			node(first, "n1").data.boardDataVersion,
		);
		// ...but nothing the node reads did, so it must not re-render.
		expect(contentVersion(second, "n1")).toBe(token);
	});
});

/**
 * `{"type":"object","additionalProperties":true}` — `flow_like::flow::pin::OPEN_OBJECT_SCHEMA`,
 * stamped on every pin built with `Pin::set_open_schema()`.
 */
const OPEN_SCHEMA = '{"type":"object","additionalProperties":true}';
const USER_SCHEMA = '{"type":"object","properties":{"sub":{"type":"string"}}}';
const OTHER_SCHEMA =
	'{"type":"object","properties":{"count":{"type":"number"}}}';

const structPin = (name: string, overrides: Partial<IPin> = {}): IPin =>
	({
		id: `pin_${name}`,
		name,
		friendly_name: name,
		description: "",
		pin_type: IPinType.Output,
		data_type: IVariableType.Struct,
		value_type: IValueType.Normal,
		depends_on: [],
		connected_to: [],
		index: 0,
		...overrides,
	}) as IPin;

describe("editor element input connections", () => {
	const editorElement = structPin("element_ref", {
		pin_type: IPinType.Input,
		data_type: IVariableType.Generic,
		options: { enforce_generic_value_type: true, enforce_schema: false },
	});

	test.each([IVariableType.Struct, IVariableType.String])(
		"accepts a normal %s output in either connection direction",
		(dataType) => {
			const output = structPin("element", { data_type: dataType });
			expect(doPinsMatch(output, editorElement, {})).toBe(true);
			expect(doPinsMatch(editorElement, output, {})).toBe(true);
		},
	);

	test("rejects arrays and execution outputs", () => {
		for (const output of [
			structPin("elements", { value_type: IValueType.Array }),
			structPin("exec", { data_type: IVariableType.Execution }),
		]) {
			expect(doPinsMatch(output, editorElement, {})).toBe(false);
			expect(doPinsMatch(editorElement, output, {})).toBe(false);
		}
	});
});

describe("stationary connection dashes", () => {
	function connectedBoard() {
		const from = getNode("from", "v1");
		const to = getNode("to", "v1");
		from.pins = {};
		to.pins = {};
		for (const dataType of [IVariableType.Struct, IVariableType.Execution]) {
			const output = structPin(`${dataType}_out`, {
				data_type: dataType,
			});
			const input = structPin(`${dataType}_in`, {
				data_type: dataType,
				pin_type: IPinType.Input,
				depends_on: [output.id],
			});
			output.connected_to = [input.id];
			from.pins[output.id] = output;
			to.pins[input.id] = input;
		}
		from.fn_refs = {
			can_reference_fns: true,
			can_be_referenced_by_fns: false,
			fn_refs: [to.id],
		};
		to.fn_refs = {
			can_reference_fns: false,
			can_be_referenced_by_fns: true,
			fn_refs: [],
		};
		return board({ nodes: { from, to }, layers: {} });
	}

	test("generated connections do not enable moving dashes", () => {
		const parsed = parse(connectedBoard());
		expect(parsed.edges).toHaveLength(3);
		for (const edge of parsed.edges as FlowSceneEdge[]) {
			expect(edge).toMatchObject({ animated: false });
			if (edge.type === "data" || edge.type === "veil") {
				expect(edgeNeedsDom(edge)).toBe(false);
			}
		}
	});

	test("cached animated edges are rebuilt once and then reused", () => {
		const source = connectedBoard();
		const initial = parse(source);
		const animated = initial.edges.map((edge) => ({
			...(edge as object),
			animated: true,
		}));
		const updated = parse(source, initial.nodes, animated);
		expect(updated.edges).toHaveLength(3);
		for (const [index, edge] of updated.edges.entries()) {
			expect(edge).not.toBe(animated[index]);
			expect(edge).toMatchObject({ animated: false });
		}
		const unchanged = parse(source, updated.nodes, updated.edges);
		for (const [index, edge] of unchanged.edges.entries()) {
			expect(edge).toBe(updated.edges[index]);
		}
	});
});

describe("boundary pin validation", () => {
	const boundaryCases = [
		["-input", IPinType.Input],
		["-return", IPinType.Output],
	] as const;

	for (const [suffix, pinType] of boundaryCases) {
		test(`${suffix} validation is repeatable with shared frozen pins in either drag direction`, () => {
			const boundary = Object.freeze(
				structPin("boundary", { pin_type: pinType }),
			);
			const peer = Object.freeze(structPin("peer", { pin_type: pinType }));
			const owner = functionLayer(`layer${suffix}`);

			for (let attempt = 0; attempt < 3; attempt++) {
				expect(doPinsMatch(boundary, peer, {}, owner)).toBe(true);
				expect(doPinsMatch(peer, boundary, {}, undefined, owner)).toBe(true);
			}
			expect(boundary.pin_type).toBe(pinType);
			expect(peer.pin_type).toBe(pinType);
		});

		test(`${suffix} uses the visible direction for asymmetric schema coverage`, () => {
			const wide =
				'{"type":"object","properties":{"name":{"type":"string"},"age":{"type":"number"}},"required":["name","age"]}';
			const narrow =
				'{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}';
			const owner = functionLayer(`layer${suffix}`);
			for (const allowed of [true, false]) {
				const boundaryIsOutput = pinType === IPinType.Input;
				const boundary = Object.freeze(
					structPin("boundary", {
						pin_type: pinType,
						schema: allowed === boundaryIsOutput ? wide : narrow,
					}),
				);
				const peer = Object.freeze(
					structPin("peer", {
						pin_type: pinType,
						schema: allowed === boundaryIsOutput ? narrow : wide,
					}),
				);
				expect(doPinsMatch(boundary, peer, {}, owner)).toBe(allowed);
				expect(doPinsMatch(peer, boundary, {}, undefined, owner)).toBe(allowed);
			}
		});

		test(`${suffix} uses the visible direction for geometry subtypes`, () => {
			const owner = functionLayer(`layer${suffix}`);
			for (const allowed of [true, false]) {
				const boundaryIsOutput = pinType === IPinType.Input;
				const boundary = Object.freeze(
					structPin("boundary", {
						pin_type: pinType,
						data_type: IVariableType.Geometry,
						schema:
							allowed === boundaryIsOutput ? geometryMarker("Point") : null,
					}),
				);
				const peer = Object.freeze(
					structPin("peer", {
						pin_type: pinType,
						data_type: IVariableType.Geometry,
						schema:
							allowed === boundaryIsOutput ? null : geometryMarker("Point"),
					}),
				);
				expect(doPinsMatch(boundary, peer, {}, owner)).toBe(allowed);
				expect(doPinsMatch(peer, boundary, {}, undefined, owner)).toBe(allowed);
			}
		});
	}

	test("generic reroute matching leaves boundary direction unchanged", () => {
		const boundary = Object.freeze(
			structPin("route_in", {
				pin_type: IPinType.Input,
				data_type: IVariableType.Generic,
			}),
		);
		expect(
			doPinsMatch(
				boundary,
				structPin("peer"),
				{},
				functionLayer("layer-input"),
			),
		).toBe(true);
		expect(boundary.pin_type).toBe(IPinType.Input);
	});

	test("the parsed inner-layer cache already has inverted pins owned by the real layer", () => {
		const input = Object.freeze(
			structPin("boundary_input", { pin_type: IPinType.Input }),
		);
		const output = Object.freeze(structPin("boundary_return"));
		const innerInput = Object.freeze(
			structPin("inner_input", { pin_type: IPinType.Input }),
		);
		const innerOutput = Object.freeze(structPin("inner_output"));
		const source = board({
			nodes: {
				inner: {
					...getNode("inner", ""),
					layer: "layer",
					pins: { [innerInput.id]: innerInput, [innerOutput.id]: innerOutput },
				},
			},
			layers: {
				layer: functionLayer("layer", {
					pins: { [input.id]: input, [output.id]: output },
				}),
			},
		});
		const { cache } = parseBoard(
			source,
			"app",
			() => {},
			() => {},
			async () => {},
			async () => {},
			new Set(),
			undefined,
			undefined,
			undefined,
			"layer",
		);
		for (const [pin] of cache.values()) Object.freeze(pin);
		for (const [sourceHandle, targetHandle] of [
			[input.id, innerInput.id],
			[innerOutput.id, output.id],
		]) {
			for (let attempt = 0; attempt < 3; attempt++) {
				expect(
					isValidConnection({ sourceHandle, targetHandle }, cache, {}),
				).toBe(true);
				expect(
					isValidConnection(
						{ sourceHandle: targetHandle, targetHandle: sourceHandle },
						cache,
						{},
					),
				).toBe(true);
			}
		}
		expect(cache.get(input.id)?.[0].pin_type).toBe(IPinType.Output);
		expect(cache.get(output.id)?.[0].pin_type).toBe(IPinType.Input);
		expect(cache.get(input.id)?.[1].id).toBe("layer");
		expect(input.pin_type).toBe(IPinType.Input);
		expect(output.pin_type).toBe(IPinType.Output);
	});

	test.each([undefined, null, ""])(
		"incomplete handle %p is rejected without throwing",
		(missing) => {
			const cache = new Map<string, [IPin, INode | ILayer, boolean]>();
			expect(
				isValidConnection(
					{ sourceHandle: missing, targetHandle: "pin" },
					cache,
					{},
				),
			).toBe(false);
			expect(
				isValidConnection(
					{ sourceHandle: "pin", targetHandle: missing },
					cache,
					{},
				),
			).toBe(false);
		},
	);
});

/**
 * A pin declaring an open shape must never veto a peer. Regression guard for the break that
 * followed `set_open_schema()` landing on `struct_in` and the whole Structs family: the pins went
 * from `schema: null` to the open marker, and the two-sided equality check in `doPinsMatch` — which
 * sits ahead of the break/make escape hatch — started rejecting every typed struct producer.
 */
describe("doPinsMatch treats an open-object schema as no schema", () => {
	test("a typed struct output connects to Break Struct's struct_in", () => {
		expect(
			doPinsMatch(
				structPin("user_context", { schema: USER_SCHEMA }),
				structPin("struct_in", {
					pin_type: IPinType.Input,
					schema: OPEN_SCHEMA,
				}),
				{},
			),
		).toBe(true);
	});

	test("an enforcing typed output still connects to struct_in", () => {
		expect(
			doPinsMatch(
				structPin("user_context", {
					schema: USER_SCHEMA,
					options: { enforce_schema: true },
				}),
				structPin("struct_in", {
					pin_type: IPinType.Input,
					schema: OPEN_SCHEMA,
				}),
				{},
			),
		).toBe(true);
	});

	test("a typed output connects to a plain 'struct' input (Get Field, Has Field)", () => {
		expect(
			doPinsMatch(
				structPin("rows", { schema: USER_SCHEMA }),
				structPin("struct", { pin_type: IPinType.Input, schema: OPEN_SCHEMA }),
				{},
			),
		).toBe(true);
	});

	test("Make Struct's open output connects to a typed struct input", () => {
		expect(
			doPinsMatch(
				structPin("struct", { schema: OPEN_SCHEMA }),
				structPin("body", { pin_type: IPinType.Input, schema: USER_SCHEMA }),
				{},
			),
		).toBe(true);
	});

	test("the open schema is recognized through a board ref", () => {
		expect(
			doPinsMatch(
				structPin("rows", { schema: USER_SCHEMA }),
				structPin("struct_in", { pin_type: IPinType.Input, schema: "ref1" }),
				{ ref1: OPEN_SCHEMA },
			),
		).toBe(true);
	});

	test("the open schema is recognized regardless of key order and whitespace", () => {
		expect(
			doPinsMatch(
				structPin("rows", { schema: USER_SCHEMA }),
				structPin("struct_in", {
					pin_type: IPinType.Input,
					schema: '{ "additionalProperties" : true , "type" : "object" }',
				}),
				{},
			),
		).toBe(true);
	});

	test("two different real schemas are still rejected", () => {
		expect(
			doPinsMatch(
				structPin("a", { schema: USER_SCHEMA }),
				structPin("b", { pin_type: IPinType.Input, schema: OTHER_SCHEMA }),
				{},
			),
		).toBe(false);
	});

	test("an object schema carrying properties is not mistaken for the open marker", () => {
		expect(
			doPinsMatch(
				structPin("a", { schema: USER_SCHEMA }),
				structPin("b", {
					pin_type: IPinType.Input,
					schema:
						'{"type":"object","additionalProperties":true,"properties":{"x":{"type":"string"}}}',
				}),
				{},
			),
		).toBe(false);
	});

	/**
	 * Cast to Struct's donor pin starts on the open marker and adopts whatever it is handed, so it
	 * belongs in the same hatch as `struct_in`/`struct_out`. The pins it is most useful with are
	 * the typed struct outputs that set `enforce_schema`, and those are exactly the ones step 9 of
	 * `doPinsMatch` refuses against a pin with no concrete schema.
	 */
	test("an enforcing typed output connects to Cast to Struct's struct_shape", () => {
		expect(
			doPinsMatch(
				structPin("user_context", {
					schema: USER_SCHEMA,
					options: { enforce_schema: true },
				}),
				structPin("struct_shape", {
					pin_type: IPinType.Input,
					schema: OPEN_SCHEMA,
					options: { enforce_schema: false },
				}),
				{},
			),
		).toBe(true);
	});

	test("struct_shape can be re-pointed at a different shape", () => {
		expect(
			doPinsMatch(
				structPin("rows", { schema: OTHER_SCHEMA }),
				structPin("struct_shape", {
					pin_type: IPinType.Input,
					schema: USER_SCHEMA,
					options: { enforce_schema: false },
				}),
				{},
			),
		).toBe(true);
	});

	test("an enforcing output still cannot reach a shapeless plain 'struct' input", () => {
		// Pre-existing behavior, unchanged: only struct_in/struct_out/struct_shape get the
		// adopt-any hatch.
		expect(
			doPinsMatch(
				structPin("user_context", {
					schema: USER_SCHEMA,
					options: { enforce_schema: true },
				}),
				structPin("struct", { pin_type: IPinType.Input, schema: OPEN_SCHEMA }),
				{},
			),
		).toBe(false);
	});

	test("a value_type mismatch still blocks the struct hatch", () => {
		expect(
			doPinsMatch(
				structPin("rows", {
					schema: USER_SCHEMA,
					value_type: IValueType.Array,
				}),
				structPin("struct_in", {
					pin_type: IPinType.Input,
					schema: OPEN_SCHEMA,
				}),
				{},
			),
		).toBe(false);
	});
});

describe("doPinsMatch accepts an output schema that covers the input schema", () => {
	const NARROW =
		'{"title":"Contact","type":"object","properties":{"sub":{"type":"string"}},"required":["sub"]}';
	const WIDE =
		'{"title":"UserContext","type":"object","properties":{"sub":{"type":"string"},"name":{"type":"string"}},"required":["sub","name"]}';

	for (const enforce_schema of [undefined, true]) {
		test(`a wider output feeds a narrower input (enforce_schema: ${enforce_schema})`, () => {
			const output = structPin("user", {
				schema: WIDE,
				options: { enforce_schema },
			});
			const input = structPin("contact", {
				pin_type: IPinType.Input,
				schema: NARROW,
				options: { enforce_schema },
			});
			expect(doPinsMatch(output, input, {})).toBe(true);
			expect(doPinsMatch(input, output, {})).toBe(true);
		});

		test(`a narrower output cannot feed a wider input (enforce_schema: ${enforce_schema})`, () => {
			const output = structPin("contact", {
				schema: NARROW,
				options: { enforce_schema },
			});
			const input = structPin("user", {
				pin_type: IPinType.Input,
				schema: WIDE,
				options: { enforce_schema },
			});
			expect(doPinsMatch(output, input, {})).toBe(false);
			expect(doPinsMatch(input, output, {})).toBe(false);
		});
	}

	test("coverage is read through board refs", () => {
		expect(
			doPinsMatch(
				structPin("user", { schema: "wide_ref" }),
				structPin("contact", {
					pin_type: IPinType.Input,
					schema: "narrow_ref",
				}),
				{ wide_ref: WIDE, narrow_ref: NARROW },
			),
		).toBe(true);
	});

	test("a covering schema still needs a matching value type", () => {
		expect(
			doPinsMatch(
				structPin("user", { schema: WIDE, value_type: IValueType.Array }),
				structPin("contact", { pin_type: IPinType.Input, schema: NARROW }),
				{},
			),
		).toBe(false);
	});
});

describe("doPinsMatch schema classification cache", () => {
	test("parses a dragged large schema once while scanning fresh candidate pins", () => {
		const schema = JSON.stringify({
			type: "object",
			properties: Object.fromEntries(
				Array.from({ length: 256 }, (_, index) => [
					`component_${index}`,
					{
						type: "object",
						properties: {
							metadata: { type: "object", additionalProperties: true },
						},
					},
				]),
			),
		});
		const output = structPin("element", { schema: "element_ref" });
		const refs = { element_ref: schema };
		const originalParse = JSON.parse;
		const parseSpy = spyOn(JSON, "parse").mockImplementation(originalParse);

		try {
			for (let index = 0; index < 128; index++) {
				const input = structPin(`candidate_${index}`, {
					pin_type: IPinType.Input,
					schema: OTHER_SCHEMA,
				});
				expect(doPinsMatch(output, input, refs)).toBe(false);
			}
			expect(
				parseSpy.mock.calls.filter(([value]) => value === schema),
			).toHaveLength(1);
		} finally {
			parseSpy.mockRestore();
		}
	});

	test("reclassifies a pin when its inline schema changes", () => {
		const output = structPin("element", { schema: OPEN_SCHEMA });
		const input = structPin("body", {
			pin_type: IPinType.Input,
			schema: OTHER_SCHEMA,
		});

		expect(doPinsMatch(output, input, {})).toBe(true);
		output.schema = USER_SCHEMA;
		expect(doPinsMatch(output, input, {})).toBe(false);
		output.schema = OPEN_SCHEMA;
		expect(doPinsMatch(output, input, {})).toBe(true);
	});

	test("reclassifies a pin when its reference changes in place", () => {
		const output = structPin("element", { schema: "element_ref" });
		const input = structPin("body", {
			pin_type: IPinType.Input,
			schema: OTHER_SCHEMA,
		});
		const refs = { element_ref: OPEN_SCHEMA };

		expect(doPinsMatch(output, input, refs)).toBe(true);
		refs.element_ref = USER_SCHEMA;
		expect(doPinsMatch(output, input, refs)).toBe(false);
		refs.element_ref = OPEN_SCHEMA;
		expect(doPinsMatch(output, input, refs)).toBe(true);
	});

	test("reclassifies a pin when a new refs map uses the same key", () => {
		const output = structPin("element", { schema: "element_ref" });
		const input = structPin("body", {
			pin_type: IPinType.Input,
			schema: OTHER_SCHEMA,
		});
		const openRefs = { element_ref: OPEN_SCHEMA };
		const concreteRefs = { element_ref: USER_SCHEMA };

		expect(doPinsMatch(output, input, openRefs)).toBe(true);
		expect(doPinsMatch(output, input, concreteRefs)).toBe(false);
		expect(doPinsMatch(output, input, openRefs)).toBe(true);
	});
});

describe("Geometry pin compatibility", () => {
	test("directional subtype rules survive reversed drag, refs, all containers, and enforcement flags", () => {
		for (const value_type of Object.values(IValueType)) {
			for (const source of [null, ...GEOMETRY_KINDS]) {
				for (const target of [null, ...GEOMETRY_KINDS]) {
					for (const enforce_schema of [undefined, true, false]) {
						const output = structPin("geometry_out", {
							data_type: IVariableType.Geometry,
							value_type,
							schema: source ? "output_ref" : null,
							options: { enforce_schema },
						});
						const input = structPin("geometry_in", {
							pin_type: IPinType.Input,
							data_type: IVariableType.Geometry,
							value_type,
							schema: target ? "input_ref" : null,
							options: { enforce_schema },
						});
						const refs = {
							output_ref: geometryMarker(source) ?? "",
							input_ref: geometryMarker(target) ?? "",
						};
						const allowed = target === null || source === target;
						expect(doPinsMatch(output, input, refs)).toBe(allowed);
						expect(doPinsMatch(input, output, refs)).toBe(allowed);
					}
				}
			}
		}
	});
	test("does not adopt Struct schemas or coerce containers", () => {
		const output = structPin("geometry_out", {
			data_type: IVariableType.Geometry,
		});
		expect(
			doPinsMatch(
				output,
				structPin("struct_in", { pin_type: IPinType.Input }),
				{},
			),
		).toBe(false);
		expect(
			doPinsMatch(
				output,
				structPin("geo_in", {
					pin_type: IPinType.Input,
					data_type: IVariableType.Geometry,
					value_type: IValueType.Array,
				}),
				{},
			),
		).toBe(false);
		expect(
			doPinsMatch(
				{ ...output, schema: '{"type":"object"}' },
				{ ...output, pin_type: IPinType.Input },
				{},
			),
		).toBe(false);
	});
});

describe("presentation updates outside the runtime node hash", () => {
	test("cached nodes follow offline and remote execution availability changes", async () => {
		const source = board();
		const boardRef = { current: source };
		const executions: INode[] = [];
		const remote = async (value: INode) => {
			executions.push(value);
		};
		const render = (
			isOffline: boolean,
			onRemoteExecute: typeof remote | undefined,
			previous?: IParsed,
		) =>
			parseBoard(
				source,
				"app-1",
				() => {},
				() => {},
				async () => {},
				async () => undefined,
				new Set<string>(),
				undefined,
				previous?.nodes,
				previous?.edges,
				undefined,
				boardRef,
				undefined,
				undefined,
				undefined,
				undefined,
				{ isOffline, onRemoteExecute },
			) as IParsed;
		const offline = render(true, remote);
		const online = render(false, remote, offline);
		expect(node(online, "n1").data.isOffline).toBe(false);
		expect(node(online, "n1").data.onRemoteExecute).toBe(
			node(offline, "n1").data.onRemoteExecute,
		);
		const unavailable = render(false, undefined, online);
		expect(node(unavailable, "n1").data.onRemoteExecute).toBeUndefined();
		const restored = render(false, remote, unavailable);
		await node(restored, "n1").data.onRemoteExecute?.(source.nodes.n1);
		expect(executions).toEqual([source.nodes.n1]);
		const unchanged = render(false, async (value) => remote(value), restored);
		expect(node(unchanged, "n1")).toBe(node(restored, "n1"));
		expect(node(offline, "n1").data.isOffline).toBe(true);
	});

	test("execution mode changes reach cached nodes without changing node hashes", () => {
		const before = board({ execution_mode: IExecutionMode.Hybrid });
		const first = parse(before);
		const second = parse(
			edit(before, { execution_mode: IExecutionMode.Remote }),
			first.nodes,
			first.edges,
		);
		expect(node(second, "n1").data.executionMode).toBe(IExecutionMode.Remote);
		expect(node(second, "n1").data.node).toBe(before.nodes.n1);
		expect(node(first, "n1").data.executionMode).toBe(IExecutionMode.Hybrid);
	});

	test("a replaced node snapshot updates its position even if its hash is unchanged", () => {
		const before = board();
		const first = parse(before);
		const moved = edit(before, {
			nodes: {
				...before.nodes,
				n1: { ...before.nodes.n1, coordinates: [140, 220, 0] },
			},
		});
		const second = parse(moved, first.nodes, first.edges);
		expect(node(second, "n1").position).toEqual({ x: 140, y: 220 });
		expect(node(first, "n1").position).toEqual({ x: 0, y: 0 });

		const dragging = {
			...node(first, "n1"),
			position: { x: 175, y: 260 },
			dragging: true,
		};
		const duringDrag = parse(moved, [dragging], first.edges);
		expect(node(duringDrag, "n1").position).toBe(dragging.position);
	});

	test("node errors appear and clear without requiring a geometry or pin edit", () => {
		const before = board();
		const first = parse(before);
		const errorNode = {
			...before.nodes.n1,
			error: "Required schema is missing",
		};
		const withError = edit(before, {
			nodes: { ...before.nodes, n1: errorNode },
		});
		const second = parse(withError, first.nodes, first.edges);
		expect(node(second, "n1").data.node.error).toBe(
			"Required schema is missing",
		);
		expect(node(second, "n1").data.node.hash).toBe(before.nodes.n1.hash);
		expect(node(second, "n1")).not.toBe(node(first, "n1"));
		const cleared = edit(withError, {
			nodes: { ...withError.nodes, n1: { ...errorNode, error: null } },
		});
		const third = parse(cleared, second.nodes, second.edges);
		expect(node(third, "n1").data.node.error).toBeNull();
		expect(node(first, "n1").data.node.error).toBeUndefined();
	});

	test("unchanged raw nodes keep their rendering while execution availability refreshes", () => {
		const before = board();
		const first = parse(before);
		const unchanged = parse(
			edit(before, { name: "Renamed board" }),
			first.nodes,
			first.edges,
		);
		expect(node(unchanged, "n1")).toBe(node(first, "n1"));
		const changed = parse(
			edit(before, {
				nodes: {
					...before.nodes,
					n1: { ...before.nodes.n1, only_offline: true },
				},
			}),
			first.nodes,
			first.edges,
		);
		expect(node(changed, "n1").data.node.only_offline).toBe(true);
	});
});
