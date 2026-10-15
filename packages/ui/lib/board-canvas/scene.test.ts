import { describe, expect, test } from "bun:test";
import type { INode } from "../schema/flow/node";
import {
	type IPin,
	IPinType,
	IValueType,
	IVariableType,
} from "../schema/flow/pin";
import {
	type FlowSceneEdge,
	type FlowSceneNode,
	type Palette,
	collectDomNodeIds,
	edgeNeedsDom,
	isNativeCanvasNode,
	nodeNeedsDom,
	sceneFromFlow,
} from "./scene";

const palette: Palette = {
	background: [10, 10, 10, 255],
	card: [20, 20, 20, 255],
	foreground: [250, 250, 250, 255],
	muted: [100, 100, 100, 255],
	border: [50, 50, 50, 255],
	primary: [0, 200, 100, 255],
	tertiary: [150, 80, 220, 255],
};

function pin(
	id: string,
	index: number,
	direction: IPinType,
	extra: Partial<IPin> = {},
): IPin {
	return {
		id,
		index,
		name: id,
		friendly_name: id,
		description: "",
		pin_type: direction,
		data_type: IVariableType.String,
		value_type: IValueType.Normal,
		connected_to: [],
		depends_on: [],
		...extra,
	};
}

function node(
	id: string,
	pins: IPin[],
	extra: Record<string, unknown> = {},
): FlowSceneNode {
	return {
		id,
		type: "node",
		position: { x: 10, y: 20 },
		data: {
			node: {
				id,
				name: "example",
				friendly_name: `Node ${id}`,
				pins: Object.fromEntries(pins.map((pin) => [pin.id, pin])),
				...extra,
			},
		},
	};
}

function edge(
	source: string,
	sourceHandle: string,
	target: string,
	targetHandle: string,
): FlowSceneEdge {
	return {
		id: `${sourceHandle}-${targetHandle}`,
		source,
		sourceHandle,
		target,
		targetHandle,
	};
}

describe("production board canvas scene", () => {
	test("persisted repeated pin groups retain add/remove controls without transient flags", () => {
		const connected = { name: "value", depends_on: ["source"] };
		const graph = node("repeated", [
			pin("a", 1, IPinType.Input, connected),
			pin("b", 2, IPinType.Input, connected),
		]);
		expect(nodeNeedsDom(graph)).toBe(true);
		expect(collectDomNodeIds([graph]).has(graph.id)).toBe(true);
		const separateDirections = node("separate", [
			pin("a", 1, IPinType.Input, connected),
			pin("b", 1, IPinType.Output, { name: "value", connected_to: ["sink"] }),
		]);
		expect(nodeNeedsDom(separateDirections)).toBe(false);
	});
	test("invalid or missing position coordinates never produce an invalid WASM scene", () => {
		const invalid = node("invalid", []);
		invalid.position = { x: Number.NaN, y: Number.POSITIVE_INFINITY };
		const stale = node("stale", []);
		stale.internals = {
			positionAbsolute: { x: Number.NaN, y: Number.NEGATIVE_INFINITY },
		};
		const boundary: FlowSceneNode = {
			id: "layer-return",
			type: "layerInnerNode",
			position: {} as FlowSceneNode["position"],
			data: { type: 1, layer: { id: "layer", name: "Layer", pins: {} } },
		};
		const scene = sceneFromFlow([invalid, stale, boundary], [], palette);
		expect(scene.nodes.map(({ x, y }) => [x, y])).toEqual([
			[0, 0],
			[10, 20],
			[200, 0],
		]);
		expect(
			JSON.parse(JSON.stringify(scene)).nodes.every(
				(node: { x: number; y: number }) =>
					Number.isFinite(node.x) && Number.isFinite(node.y),
			),
		).toBe(true);
	});
	test("production controls and state remain mounted without selecting their nodes", () => {
		const baseline = node("baseline", [
			pin("out", 1, IPinType.Output, { connected_to: ["in"] }),
		]);
		expect(nodeNeedsDom(baseline)).toBe(false);
		expect(nodeNeedsDom({ ...baseline, zIndex: 20 })).toBe(false);
		expect(nodeNeedsDom({ ...baseline, zIndex: 30 })).toBe(true);
		expect(nodeNeedsDom({ ...baseline, style: { opacity: 0.15 } })).toBe(true);
		expect(nodeNeedsDom({ ...baseline, className: "preview-member" })).toBe(
			true,
		);
		expect(
			nodeNeedsDom({ ...baseline, style: { width: 220, height: 80 } }),
		).toBe(true);
		for (const extra of [
			{ error: "Runtime failed" },
			{ comment: "Important context" },
			{ icon: "/icons/catalog.svg" },
			{ long_running: true },
			{ only_offline: true },
			{ start: true },
			{ event_callback: true },
			{ wasm: { package_id: "package" } },
			{ fn_refs: { can_reference_fns: true } },
		])
			expect(nodeNeedsDom(node("rich", [], extra))).toBe(true);
		expect(nodeNeedsDom(node("latch", [pin("out", 1, IPinType.Output)]))).toBe(
			true,
		);
		expect(
			nodeNeedsDom(
				node("editor", [
					pin("bool", 1, IPinType.Input, { data_type: IVariableType.Boolean }),
				]),
			),
		).toBe(true);
		expect(
			nodeNeedsDom(
				node("container", [
					pin("list", 1, IPinType.Output, {
						value_type: IValueType.Array,
						connected_to: ["in"],
					}),
				]),
			),
		).toBe(true);
		for (const data of [
			{ remoteSelections: [{ userId: "peer" }] },
			{ remoteEditors: [{ userId: "peer" }] },
			{ remoteExecuting: true },
			{ isUnavailable: true },
			{ functionCache: { ttl: 60 } },
		])
			expect(
				nodeNeedsDom({ ...baseline, data: { ...baseline.data, ...data } }),
			).toBe(true);
	});

	test("runtime projection follows quality, run, log, and heatmap stores", () => {
		const nodes = [
			"idle",
			"running",
			"done",
			"logged",
			"summary",
			"heat",
			"lint",
		].map((id) => {
			const value = node(id, []);
			value.data.boardId = "board";
			return value;
		});
		const state = {
			runs: new Map([
				[
					"run",
					{ nodes: new Set(["running"]), already_executed: new Set(["done"]) },
				],
			]),
			currentMetadata: { run_id: "run", nodes: [["logged", 3]] },
			currentSummaryRunId: "run",
			currentSummary: { nodes: { summary: [0, 0, 1, 0] } },
			heatmapEnabled: true,
			heatmap: { nodes: { heat: { visits: 2 } } },
			qualityInline: true,
			qualityMarks: { board: { lint: { severity: "error" } } },
		};
		expect([...collectDomNodeIds(nodes, state)]).toEqual([
			"running",
			"done",
			"logged",
			"summary",
			"heat",
			"lint",
		]);
		expect([
			...collectDomNodeIds(nodes, {
				...state,
				currentSummaryRunId: "stale",
				heatmapEnabled: false,
				qualityInline: false,
			}),
		]).toEqual(["running", "done", "logged"]);
		const ids = collectDomNodeIds(nodes, state);
		expect(
			sceneFromFlow(nodes, [], palette, { domNodeIds: ids })
				.nodes.filter((node) => node.dom)
				.map((node) => node.id),
		).toEqual([...ids]);
	});

	test("native edges defer animations, path variants and decorations to the original SVG", () => {
		const base = {
			...edge("a", "out", "b", "in"),
			type: "execution",
			data: { reduceMotion: true },
			style: { stroke: "var(--foreground)" },
		};
		expect(edgeNeedsDom(base)).toBe(false);
		expect(edgeNeedsDom({ ...base, data: {} })).toBe(true);
		expect(edgeNeedsDom({ ...base, data: {} }, true)).toBe(false);
		for (const change of [
			{ data: { pathType: "straight", reduceMotion: true } },
			{ data: { pathType: "step", reduceMotion: true } },
			{ animated: true },
			{ className: "opacity-10" },
			{ zIndex: 30 },
			{ markerEnd: "url(#arrow)" },
			{ label: "Runs after" },
			{ style: { stroke: "var(--destructive)" } },
			{ style: { opacity: 0.2 } },
		])
			expect(edgeNeedsDom({ ...base, ...change })).toBe(true);
	});

	test("the production flowNode alias preserves real geometry", () => {
		const aliased = { ...node("alias", []), type: "flowNode" };
		expect(isNativeCanvasNode(aliased)).toBe(true);
		expect(sceneFromFlow([aliased], [], palette).nodes[0].id).toBe("alias");
	});

	test("explicit theme strokes keep their color when an edge is selected", () => {
		const source = node("a", [pin("out", 1, IPinType.Output)]);
		const target = node("b", [pin("in", 1, IPinType.Input)]);
		const selected = {
			...edge("a", "out", "b", "in"),
			selected: true,
			type: "data",
			style: { stroke: "var(--pin-boolean)" },
		};
		const scene = sceneFromFlow([source, target], [selected], palette, {
			pinColors: { String: [1, 2, 3, 255], Boolean: [4, 5, 6, 255] },
			webkitLite: true,
		});
		expect(scene.edges[0]).toMatchObject({
			selected: true,
			color: [4, 5, 6, 255],
			lite: true,
			dom: false,
		});
	});

	test("dynamic pin families keep canonical ids, indices and connection endpoints", () => {
		const source = node("source", [pin("out", 1, IPinType.Output)]);
		const dynamic = node("dynamic", [
			pin("second", 2, IPinType.Input, { name: "item", depends_on: ["out"] }),
			pin("third", 3, IPinType.Input, { name: "item" }),
			pin("first", 1, IPinType.Input, { name: "item" }),
		]);
		const before = JSON.stringify(dynamic);
		const scene = sceneFromFlow(
			[source, dynamic],
			[edge("source", "out", "dynamic", "second")],
			palette,
		);
		expect(scene.nodes[1].pins.map((pin) => [pin.id, pin.y])).toEqual([
			["first", 28],
			["second", 43],
			["third", 58],
		]);
		// The fourth row reserves the production family's add-pin control, without inventing a graph pin.
		expect(scene.nodes[1].height).toBe(98);
		expect(scene.edges[0].target).toEqual({
			nodeId: "dynamic",
			pinId: "second",
		});
		expect(JSON.stringify(dynamic)).toBe(before);
	});

	test("collapsed pins retain execution and connected pins and repack only presentation slots", () => {
		const graphNode = node(
			"collapsed",
			[
				pin("exec", 1, IPinType.Input, { data_type: IVariableType.Execution }),
				pin("loose", 2, IPinType.Input),
				pin("wired", 5, IPinType.Input, { depends_on: ["other"] }),
				pin("out", 4, IPinType.Output, { connected_to: ["target"] }),
			],
			{ pins_collapsed: true },
		);
		const before = JSON.stringify(graphNode);
		const compact = sceneFromFlow([graphNode], [], palette).nodes[0];
		expect(compact.pins.map((pin) => [pin.id, pin.y])).toEqual([
			["exec", 28],
			["wired", 43],
			["out", 28],
		]);
		const expanded = sceneFromFlow([graphNode], [], palette, {
			collapsed: new Map([["collapsed", false]]),
		}).nodes[0];
		expect(expanded.pins.find((pin) => pin.id === "wired")?.y).toBe(88);
		expect(expanded.pins.some((pin) => pin.id === "loose")).toBe(true);
		expect(JSON.stringify(graphNode)).toBe(before);
	});

	test("call functions hide their bound layer pin and compact remaining input rows without mutation", () => {
		const call = node(
			"call",
			[
				pin("ref", 1, IPinType.Input, { name: "function_layer_id" }),
				pin("exec", 2, IPinType.Input, {
					name: "exec_in",
					data_type: IVariableType.Execution,
				}),
				pin("arg", 3, IPinType.Input),
				pin("result", 2, IPinType.Output),
			],
			{ name: "control_call_function" },
		);
		call.type = "callFunctionNode";
		const before = JSON.stringify(call);
		const scene = sceneFromFlow([call], [], palette);
		expect(scene.nodes[0].kind).toBe("function");
		expect(
			scene.nodes[0].pins.map((pin) => [pin.id, pin.y, pin.label]),
		).toEqual([
			["exec", 28, ""],
			["arg", 43, "arg"],
			["result", 43, "result"],
		]);
		expect(JSON.stringify(call)).toBe(before);
	});

	test("layer proxies retain their visible node ids and already-inverted boundary direction", () => {
		const boundary: FlowSceneNode = {
			id: "layer-input",
			type: "layerInnerNode",
			position: { x: 0, y: 0 },
			data: {
				type: 0,
				layer: {
					id: "layer",
					name: "Function",
					pins: { entry: pin("entry", 1, IPinType.Output) },
				},
			},
		};
		const proxy: FlowSceneNode = {
			id: "child-layer",
			type: "layerNode",
			position: { x: 300, y: 0 },
			data: {
				layer: {
					id: "child-layer",
					name: "Group",
					pins: { original: pin("original", 1, IPinType.Input) },
				},
			},
		};
		const before = JSON.stringify(boundary);
		const scene = sceneFromFlow(
			[boundary, proxy],
			[edge("layer-input", "entry", "child-layer", "original")],
			palette,
		);
		expect(scene.nodes[0].pins[0].direction).toBe("output");
		expect(scene.nodes[0].label).toBe('"Function" Start');
		expect(scene.edges[0]).toMatchObject({
			source: { nodeId: "layer-input", pinId: "entry" },
			target: { nodeId: "child-layer", pinId: "original" },
		});
		expect(
			sceneFromFlow([boundary], [], palette).nodes[0].pins[0].direction,
		).toBe("output");
		expect(JSON.stringify(boundary)).toBe(before);
	});

	test("function-reference handles have their own ids, geometry and edge kind", () => {
		const from = node("from", [], {
			fn_refs: {
				can_reference_fns: true,
				can_be_referenced_by_fns: false,
				fn_refs: ["to"],
			},
		});
		const to = node("to", [], {
			fn_refs: {
				can_reference_fns: false,
				can_be_referenced_by_fns: true,
				fn_refs: [],
			},
		});
		const refEdge = {
			...edge("from", "ref_out_from", "to", "ref_in_to"),
			type: "veil",
			selected: true,
		};
		const scene = sceneFromFlow([from, to], [refEdge], palette, {
			referenceColor: [1, 2, 3, 255],
		});
		expect(scene.nodes[0].pins[0]).toMatchObject({
			id: "ref_out_from",
			direction: "output",
			x: 75,
			y: scene.nodes[0].height,
			kind: "reference",
		});
		expect(scene.nodes[1].pins[0]).toMatchObject({
			id: "ref_in_to",
			direction: "input",
			x: 75,
			y: 0,
		});
		expect(scene.edges[0]).toMatchObject({
			kind: "reference",
			selected: true,
			color: [1, 2, 3, 255],
		});
	});

	test("measured bounds and handle centers override fallback geometry in graph coordinates", () => {
		const measured = node("measured", [pin("input", 1, IPinType.Input)]);
		measured.measured = { width: 180, height: 90 };
		measured.internals = { positionAbsolute: { x: 400, y: 500 } };
		const scene = sceneFromFlow([measured], [], palette, {
			geometry: new Map([
				[
					"measured",
					{
						width: 200,
						handles: [
							{
								id: "input",
								direction: "input",
								x: -6,
								y: 33,
								width: 12,
								height: 12,
							},
						],
					},
				],
			]),
		});
		expect(scene.nodes[0]).toMatchObject({
			x: 400,
			y: 500,
			width: 200,
			height: 90,
		});
		expect(scene.nodes[0].pins[0]).toMatchObject({ x: 0, y: 39 });
	});

	test("reroutes use compact dimensions and centered pins, and start-node inputs stay hidden", () => {
		const reroute = node(
			"route",
			[pin("in", 1, IPinType.Input), pin("out", 1, IPinType.Output)],
			{ name: "reroute", pins_collapsed: true },
		);
		const start = node(
			"start",
			[pin("hidden", 1, IPinType.Input), pin("visible", 1, IPinType.Output)],
			{ start: true },
		);
		const scene = sceneFromFlow([reroute, start], [], palette);
		expect(scene.nodes[0]).toMatchObject({
			width: 16,
			height: 12,
			kind: "reroute",
		});
		expect(scene.nodes[0].pins.map((pin) => [pin.x, pin.y, pin.label])).toEqual(
			[
				[0, 6, ""],
				[16, 6, ""],
			],
		);
		expect(scene.nodes[1].pins.map((pin) => pin.id)).toEqual(["visible"]);
		expect(scene.nodes[1].kind).toBe("event");
	});

	test("comments and media stay with the DOM host, and dangling edges get no invented endpoints", () => {
		const graphNode = node("graph", [pin("out", 1, IPinType.Output)]);
		const comment = {
			id: "comment",
			type: "commentNode",
			position: { x: 0, y: 0 },
			data: { comment: { content: "rich" } },
		};
		const media = { ...comment, id: "media", type: "mediaNode" };
		expect(isNativeCanvasNode(comment)).toBe(false);
		expect(isNativeCanvasNode(media)).toBe(false);
		expect(isNativeCanvasNode({ type: "node", data: {} })).toBe(false);
		const scene = sceneFromFlow(
			[graphNode, comment, media],
			[edge("graph", "out", "media", "missing")],
			palette,
		);
		expect(scene.nodes.map((node) => node.id)).toEqual(["graph"]);
		expect(scene.edges).toEqual([]);
	});

	test("resolved type colors, label caps and selection are retained without browser APIs", () => {
		const typed = node("typed", [
			pin("short", 1, IPinType.Input),
			pin("long", 1, IPinType.Output, {
				friendly_name: "A considerably longer output label",
			}),
		]);
		typed.selected = true;
		typed.data.isUnavailable = true;
		const scene = sceneFromFlow([typed], [], palette, {
			pinColors: { String: [123, 45, 67, 255] },
		});
		expect(scene.nodes[0]).toMatchObject({ selected: true, dimmed: true });
		expect(scene.nodes[0].pins[0].color).toEqual([123, 45, 67, 255]);
		expect(
			(scene.nodes[0].pins[0].labelMaxWidth ?? Number.NaN) +
				(scene.nodes[0].pins[1].labelMaxWidth ?? Number.NaN),
		).toBe(134);
	});

	test("pin typography and geometry follow rem sizing while headers keep production fitting", () => {
		const graph = node("fitting", [pin("label", 1, IPinType.Output)]);
		(graph.data.node as INode).friendly_name =
			"A workflow title with substantially more than thirty characters";
		const normal = sceneFromFlow([graph], [], palette).nodes[0];
		const large = sceneFromFlow([graph], [], palette, { rootFontSize: 20 })
			.nodes[0];
		expect(normal.pins[0].fontSize).toBe(9.6);
		expect(large.pins[0].fontSize).toBe(12);
		expect(large.pins[0].y - normal.pins[0].y).toBe(7);
		expect(large.height - normal.height).toBe(7);
		expect(large.headerHeight).toBe(20);
		expect(normal.labelFontSize).toBe(6);
		expect(large.labelFontSize).toBe(normal.labelFontSize);
	});

	test("node and reroute corner radii follow the resolved CSS including square styles", () => {
		const nodes = [
			node("ordinary", []),
			node("route", [], { name: "reroute" }),
		];
		expect(
			sceneFromFlow(nodes, [], palette).nodes.map((node) => node.cornerRadius),
		).toEqual([4, 6]);
		expect(
			sceneFromFlow(nodes, [], palette, {
				nodeRadius: 10,
				rerouteRadius: 12,
			}).nodes.map((node) => node.cornerRadius),
		).toEqual([10, 12]);
		expect(
			sceneFromFlow(nodes, [], palette, {
				nodeRadius: 0,
				rerouteRadius: 0,
			}).nodes.map((node) => node.cornerRadius),
		).toEqual([0, 0]);
	});

	test("unsupported glyphs and custom fonts preserve complete browser text rendering", () => {
		const graph = node("translated", [pin("out", 1, IPinType.Output)]);
		(graph.data.node as INode).pins.out.connected_to = ["peer"];
		expect(nodeNeedsDom(graph)).toBe(false);
		(graph.data.node as INode).friendly_name = "日本語の処理";
		expect(nodeNeedsDom(graph)).toBe(true);
		(graph.data.node as INode).friendly_name = "Supported title";
		(graph.data.node as INode).pins.out.friendly_name = "Result 😀";
		expect(nodeNeedsDom(graph)).toBe(true);
		(graph.data.node as INode).pins.out.friendly_name = "Result";
		expect(collectDomNodeIds([graph], { forceDomText: true })).toEqual(
			new Set(["translated"]),
		);
	});
});
