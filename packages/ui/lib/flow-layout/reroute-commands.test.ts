import { describe, expect, test } from "bun:test";
import { moveNodeCommand } from "../command/generic-command";
import {
	ICommandType,
	type IGenericCommand,
} from "../schema/flow/board/commands/generic-command";
import type { INode, IPin } from "../schema/flow/node";
import { IPinType, IValueType, IVariableType } from "../schema/flow/node";
import type { ILayer } from "../schema/flow/run";
import { computeFlowLayout, computeFlowLayoutDetailed } from "./index";
import {
	type AutoRerouteChain,
	normalizeAutoReroutes,
} from "./normalize-reroutes";
import { buildAutoRerouteCommands } from "./reroute-commands";
import type { WireRoute } from "./route";
import { GraphBuilder } from "./test-fixtures";
import type { AutoLayoutInput, LayoutStyle } from "./types";
function template(): INode {
	return {
		id: "template",
		name: "reroute",
		friendly_name: "Reroute",
		category: "Control",
		description: "",
		pins: Object.fromEntries(
			["route_in", "route_out"].map((name, index) => [
				name,
				{
					id: name,
					name,
					friendly_name: name,
					description: "",
					pin_type: index ? IPinType.Output : IPinType.Input,
					data_type: IVariableType.Generic,
					value_type: IValueType.Normal,
					index: 1,
					connected_to: [],
					depends_on: [],
				},
			]),
		),
	};
}
function cache(input: AutoLayoutInput) {
	const result = new Map<string, [IPin, INode | ILayer, boolean]>();
	for (const node of input.layerNodes) {
		for (const pin of Object.values(node.pins))
			result.set(pin.id, [pin, node, true]);
	}
	return result;
}
function fixture(): AutoLayoutInput {
	const graph = new GraphBuilder();
	graph.exec("event", { start: true, execIn: false, dataOuts: 1 });
	graph.exec("middle", { dataIns: 1, dataOuts: 2 });
	graph.exec("target", { dataIns: 1 });
	graph.execLink("event", "middle");
	graph.execLink("middle", "target");
	graph.dataLink("event", "target");
	graph.dataLink("event", "middle");
	const input: AutoLayoutInput = {
		layerNodes: [...graph.nodes.values()],
		layerEntities: [],
		currentLayer: undefined,
	};
	const pins = cache(input);
	for (const [pin] of pins.values()) {
		for (const target of pin.connected_to)
			pins.get(target)?.[0].depends_on.push(pin.id);
	}
	return input;
}
function executionFixture(): AutoLayoutInput {
	const graph = new GraphBuilder();
	graph.exec("branch", {
		start: true,
		execIn: false,
		execOuts: ["true", "false"],
	});
	graph.exec("middle");
	graph.exec("target", { execOuts: 0 });
	graph.execLink("branch", "middle", "true");
	graph.execLink("branch", "target", "false");
	graph.execLink("middle", "target");
	const input = graph.build();
	const pins = cache(input);
	for (const [pin] of pins.values()) {
		for (const target of pin.connected_to)
			pins.get(target)?.[0].depends_on.push(pin.id);
	}
	return input;
}

function sharedFixture(): { input: AutoLayoutInput; routes: WireRoute[] } {
	const graph = new GraphBuilder();
	graph.pure("source", { dataIns: 0, dataOuts: 2 });
	for (const id of ["alpha", "beta", "gamma", "sibling", "other"])
		graph.pure(id, { dataOuts: 0 });
	for (const id of ["alpha", "beta", "gamma", "sibling"])
		graph.dataLink("source", id);
	graph.dataLink("source", "other", 1);
	const input = graph.build();
	const pins = cache(input);
	for (const [pin] of pins.values()) {
		for (const target of pin.connected_to)
			pins.get(target)?.[0].depends_on.push(pin.id);
	}
	const trunk = { x: 200, y: -80, shared: "entry" };
	const branch = { x: 400, y: -80, shared: "branch" };
	const routes: WireRoute[] = [
		{
			from: "source",
			to: "alpha",
			fromPin: "source:out-0",
			toPin: "alpha:in-0",
			waypoints: [trunk, branch],
		},
		{
			from: "source",
			to: "beta",
			fromPin: "source:out-0",
			toPin: "beta:in-0",
			waypoints: [trunk, branch, { x: 600, y: -80 }],
		},
		{
			from: "source",
			to: "gamma",
			fromPin: "source:out-0",
			toPin: "gamma:in-0",
			waypoints: [trunk, { x: 300, y: -40 }],
		},
	];
	return { input, routes };
}

// Trace physical inputs to check that every shared branch still reaches its source.
function chainsFor(
	input: AutoLayoutInput,
	routes: WireRoute[],
): AutoRerouteChain[] {
	const pins = cache(input);
	return routes.map((route) => {
		let current = required(pins.get(route.toPin))[0];
		const nodes: INode[] = [];
		const seen = new Set<string>();
		while (!current.depends_on.includes(route.fromPin)) {
			expect(current.depends_on).toHaveLength(1);
			const [, owner] = required(pins.get(current.depends_on[0]));
			const node = owner as INode;
			expect(node.auto_reroute).toBe(true);
			expect(seen.has(node.id)).toBe(false);
			seen.add(node.id);
			nodes.unshift(node);
			current = required(
				Object.values(node.pins).find((pin) => pin.name === "route_in"),
			);
		}
		return { ...route, nodes };
	});
}

function topology(input: AutoLayoutInput) {
	return input.layerNodes
		.map((node) => ({
			id: node.id,
			coordinates: node.coordinates,
			layer: node.layer,
			auto_reroute: node.auto_reroute,
			pins: Object.values(node.pins)
				.map((pin) => ({
					...pin,
					connected_to: [...pin.connected_to].sort(),
					depends_on: [...pin.depends_on].sort(),
				}))
				.sort((a, b) => a.id.localeCompare(b.id)),
		}))
		.sort((a, b) => a.id.localeCompare(b.id));
}

function inverse(commands: IGenericCommand[]): IGenericCommand[] {
	return [...commands].reverse().map((command) => {
		switch (command.command_type) {
			case ICommandType.AddNode:
				return { ...command, command_type: ICommandType.RemoveNode };
			case ICommandType.RemoveNode:
				return {
					...command,
					command_type: ICommandType.AddNode,
					current_layer: command.node?.layer,
				};
			case ICommandType.ConnectPin:
				return { ...command, command_type: ICommandType.DisconnectPin };
			case ICommandType.DisconnectPin:
				return { ...command, command_type: ICommandType.ConnectPin };
			case ICommandType.MoveNode:
				return {
					...command,
					from_coordinates: command.to_coordinates,
					to_coordinates: required(command.from_coordinates),
				};
			default:
				throw new Error(`Unexpected command ${command.command_type}`);
		}
	});
}
// Applies the public command payload to a detached board. Rust batch tests exercise undo.
function apply(
	input: AutoLayoutInput,
	commands: IGenericCommand[],
): AutoLayoutInput {
	const next = structuredClone(input);
	const nodes = new Map(next.layerNodes.map((node) => [node.id, node]));
	for (const command of commands) {
		switch (command.command_type) {
			case ICommandType.MoveNode:
				required(nodes.get(required(command.node_id))).coordinates = [
					...required(command.to_coordinates),
				];
				break;
			case ICommandType.AddNode:
				nodes.set(required(command.node).id, {
					...structuredClone(required(command.node)),
					layer: command.current_layer,
				});
				break;
			case ICommandType.RemoveNode:
				nodes.delete(required(command.node).id);
				break;
			case ICommandType.ConnectPin: {
				const source = required(nodes.get(required(command.from_node))).pins[
					required(command.from_pin)
				];
				const target = required(nodes.get(required(command.to_node))).pins[
					required(command.to_pin)
				];
				if (source.data_type === IVariableType.Execution) {
					source.connected_to = [target.id];
					for (const node of nodes.values()) {
						for (const pin of Object.values(node.pins))
							pin.depends_on = pin.depends_on.filter((id) => id !== source.id);
					}
					target.depends_on.push(source.id);
				} else {
					source.connected_to = [
						...new Set([...source.connected_to, target.id]),
					];
					target.depends_on = [source.id];
				}
				break;
			}
			case ICommandType.DisconnectPin: {
				const source = required(nodes.get(required(command.from_node))).pins[
					required(command.from_pin)
				];
				const target = required(nodes.get(required(command.to_node))).pins[
					required(command.to_pin)
				];
				source.connected_to = source.connected_to.filter(
					(id) => id !== target.id,
				);
				target.depends_on = target.depends_on.filter((id) => id !== source.id);
				break;
			}
			default:
				throw new Error(`Unexpected command ${command.command_type}`);
		}
	}
	next.layerNodes = [...nodes.values()];
	return next;
}
function layoutCommands(input: AutoLayoutInput, style: LayoutStyle = "routed") {
	const result = computeFlowLayoutDetailed(input, style);
	const commands = [...result.positions].map(([id, coordinates]) =>
		moveNodeCommand({
			node_id: id,
			to_coordinates: [...coordinates, 0],
			current_layer: input.currentLayer,
		}),
	);
	if (result.routing)
		commands.push(
			...buildAutoRerouteCommands({
				...result.routing,
				reroute: style === "routed" ? template() : undefined,
				currentLayer: input.currentLayer,
				pinCache: cache(input),
			}),
		);
	return { result, commands };
}
const logicalEdges = (input: AutoLayoutInput) =>
	normalizeAutoReroutes(input)
		.input.layerNodes.flatMap((node) =>
			Object.values(node.pins).flatMap((pin) =>
				pin.connected_to.map((id) => `${pin.id}->${id}`),
			),
		)
		.sort();
describe("automatic reroute graph edits", () => {
	test("inserts, reuses, and removes execution chains while preserving other incoming wires", () => {
		const input = executionFixture();
		const route = {
			from: "branch",
			to: "target",
			fromPin: "branch:false",
			toPin: "target:exec-in",
			waypoints: [
				{ x: 250, y: 120 },
				{ x: 550, y: 120 },
			],
		};
		const commands = buildAutoRerouteCommands({
			routes: [route],
			chains: [],
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(input),
		});
		const routed = apply(input, commands);
		const dots = routed.layerNodes.filter((node) => node.auto_reroute);
		expect(dots).toHaveLength(2);
		for (const dot of dots) {
			for (const pin of Object.values(dot.pins)) {
				expect(pin.data_type).toBe(IVariableType.Execution);
				expect(pin.default_value).toBeNull();
			}
		}
		expect(cache(routed).get("target:exec-in")?.[0].depends_on).toContain(
			"middle:exec-out-0",
		);
		expect(logicalEdges(routed)).toEqual(logicalEdges(input));
		const normalized = normalizeAutoReroutes(routed);
		expect(normalized.chains).toHaveLength(1);
		expect(normalized.input.layerNodes).toHaveLength(input.layerNodes.length);
		for (const current of [routed, JSON.parse(JSON.stringify(routed))]) {
			expect(
				buildAutoRerouteCommands({
					routes: [route],
					chains: normalizeAutoReroutes(current).chains,
					reroute: template(),
					currentLayer: undefined,
					pinCache: cache(current),
				}),
			).toEqual([]);
		}
		const removed = buildAutoRerouteCommands({
			routes: [{ ...route, waypoints: [] }],
			chains: normalized.chains,
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(routed),
		});
		const direct = apply(routed, removed);
		expect(direct.layerNodes.some((node) => node.auto_reroute)).toBe(false);
		expect(logicalEdges(direct)).toEqual(logicalEdges(input));
		expect(cache(direct).get("target:exec-in")?.[0].depends_on.sort()).toEqual([
			"branch:false",
			"middle:exec-out-0",
		]);
	});
	test("keeps generated execution reroutes that merge incoming wires", () => {
		const input = executionFixture();
		const commands = buildAutoRerouteCommands({
			routes: [
				{
					from: "branch",
					to: "target",
					fromPin: "branch:false",
					toPin: "target:exec-in",
					waypoints: [{ x: 250, y: 120 }],
				},
			],
			chains: [],
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(input),
		});
		const routed = apply(input, commands);
		const pins = cache(routed);
		const dot = required(routed.layerNodes.find((node) => node.auto_reroute));
		const routeIn = required(
			Object.values(dot.pins).find((pin) => pin.name === "route_in"),
		);
		required(pins.get("middle:exec-out-0"))[0].connected_to = [routeIn.id];
		required(pins.get("target:exec-in"))[0].depends_on = required(
			pins.get("target:exec-in"),
		)[0].depends_on.filter((id) => id !== "middle:exec-out-0");
		routeIn.depends_on.push("middle:exec-out-0");
		expect(normalizeAutoReroutes(routed).chains).toEqual([]);
		expect(normalizeAutoReroutes(routed).input).toBe(routed);
	});
	test("keeps chains between the two boundaries of the same open layer", () => {
		const input = fixture();
		const routed = apply(input, layoutCommands(input).commands);
		routed.currentLayer = "fn";
		required(routed.layerNodes.find((node) => node.id === "event")).id =
			"fn-input";
		required(routed.layerNodes.find((node) => node.id === "target")).id =
			"fn-return";
		const normalized = normalizeAutoReroutes(routed);
		expect(normalized.chains).toEqual([]);
		expect(normalized.input).toBe(routed);
		const next = layoutCommands(routed);
		expect(
			next.commands.every(
				(command) => command.command_type === ICommandType.MoveNode,
			),
		).toBe(true);
	});
	test.each([
		["data", fixture],
		["execution", executionFixture],
	] as const)(
		"routes %s skips, preserves connections, and reuses identical nodes on repeat",
		(_, makeFixture) => {
			const input = makeFixture();
			const original = structuredClone(input);
			const first = layoutCommands(input);
			const routed = apply(input, first.commands);
			const dots = routed.layerNodes.filter((node) => node.auto_reroute);
			expect(dots.length).toBeGreaterThanOrEqual(2);
			expect(logicalEdges(routed)).toEqual(logicalEdges(input));
			expect(input).toEqual(original);
			const second = layoutCommands(routed);
			expect(
				second.commands.some(
					(command) => command.command_type === ICommandType.AddNode,
				),
			).toBe(false);
			expect(
				second.commands.some(
					(command) => command.command_type === ICommandType.RemoveNode,
				),
			).toBe(false);
			expect(apply(routed, second.commands)).toEqual(routed);
			const restored = JSON.parse(JSON.stringify(routed)) as AutoLayoutInput;
			expect(apply(restored, layoutCommands(restored).commands)).toEqual(
				restored,
			);
		},
	);
	test("preserves an existing chain when no route could be resolved", () => {
		const input = fixture();
		const routed = apply(input, layoutCommands(input).commands);
		const normalized = normalizeAutoReroutes(routed);
		expect(normalized.chains.length).toBeGreaterThan(0);
		const commands = buildAutoRerouteCommands({
			routes: normalized.chains.map((chain) => ({
				...chain,
				waypoints: [],
				unresolved: true,
			})),
			chains: normalized.chains,
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(routed),
		});
		expect(commands).toEqual([]);
		expect(apply(routed, commands)).toEqual(routed);
	});
	test("preserves normalized chains omitted from the routing plan", () => {
		const input = fixture();
		const routed = apply(input, layoutCommands(input).commands);
		const normalized = normalizeAutoReroutes(routed);
		expect(normalized.chains.length).toBeGreaterThan(0);
		const commands = buildAutoRerouteCommands({
			routes: [],
			chains: normalized.chains,
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(routed),
		});
		expect(commands).toEqual([]);
		expect(apply(routed, commands)).toEqual(routed);
	});
	test("shrinks a generated chain without dropping sibling connections", () => {
		const input = fixture();
		const routed = apply(input, layoutCommands(input).commands);
		const normalized = normalizeAutoReroutes(routed);
		const commands = buildAutoRerouteCommands({
			routes: normalized.chains.map((chain) => ({ ...chain, waypoints: [] })),
			chains: normalized.chains,
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(routed),
		});
		const direct = apply(routed, commands);
		expect(direct.layerNodes.some((node) => node.auto_reroute)).toBe(false);
		expect(logicalEdges(direct)).toEqual(logicalEdges(input));
		const firstRemove = commands.findIndex(
			(command) => command.command_type === ICommandType.RemoveNode,
		);
		expect(
			commands
				.slice(0, firstRemove)
				.every(
					(command) => command.command_type === ICommandType.DisconnectPin,
				),
		).toBe(true);
	});
	test("copies data types and schema onto every generated pin", () => {
		const input = fixture();
		const source = input.layerNodes[0].pins["event:out-0"];
		source.data_type = IVariableType.Struct;
		source.value_type = IValueType.Array;
		source.schema = "message-history";
		const routed = apply(input, layoutCommands(input).commands);
		for (const node of routed.layerNodes.filter((node) => node.auto_reroute)) {
			for (const pin of Object.values(node.pins)) {
				expect(pin.data_type).toBe(IVariableType.Struct);
				expect(pin.value_type).toBe(IValueType.Array);
				expect(pin.schema).toBe("message-history");
			}
		}
	});
	test("preserves manual reroutes and branched generated reroutes", () => {
		const input = fixture();
		const routed = apply(input, layoutCommands(input).commands);
		const dot = required(routed.layerNodes.find((node) => node.auto_reroute));
		dot.auto_reroute = false;
		expect(
			normalizeAutoReroutes(routed).input.layerNodes.some(
				(node) => node.id === dot.id,
			),
		).toBe(true);
		dot.auto_reroute = true;
		const output = required(
			Object.values(dot.pins).find((pin) => pin.pin_type === "Output"),
		);
		output.connected_to.push("external-input");
		expect(
			normalizeAutoReroutes(routed).input.layerNodes.some(
				(node) => node.id === dot.id,
			),
		).toBe(true);
	});
	test("includes generated chains between selected endpoints but leaves boundary chains intact", () => {
		const input = fixture();
		const routed = apply(input, layoutCommands(input).commands);
		const dot = required(routed.layerNodes.find((node) => node.auto_reroute));
		const selected = {
			...routed,
			only: new Set(["event", "target"]),
			obstacles: [
				{
					id: dot.id,
					x: required(dot.coordinates)[0],
					y: required(dot.coordinates)[1],
					width: 16,
					height: 12,
				},
			],
		};
		const normalized = normalizeAutoReroutes(selected);
		expect(normalized.chains.length).toBeGreaterThan(0);
		expect(normalized.input.obstacles).toEqual([]);
		const boundary = normalizeAutoReroutes({
			...routed,
			only: new Set(["event", "middle"]),
		});
		expect(boundary.chains).toEqual([]);
		expect(boundary.input.layerNodes.length).toBe(routed.layerNodes.length);
	});
	test("uses real pin owners when displayed endpoints are layer boundaries", () => {
		const input = fixture();
		const pinCache = cache(input);
		const [source] = required(pinCache.get("event:out-0"));
		pinCache.set(source.id, [source, { id: "real-layer" } as ILayer, true]);
		const commands = buildAutoRerouteCommands({
			routes: [
				{
					from: "real-layer-input",
					to: "target",
					fromPin: source.id,
					toPin: "target:in-0",
					waypoints: [{ x: 250, y: 120 }],
				},
			],
			chains: [],
			reroute: template(),
			currentLayer: "real-layer",
			pinCache,
		});
		expect(
			commands.find(
				(command) => command.command_type === ICommandType.DisconnectPin,
			)?.from_node,
		).toBe("real-layer");
		expect(
			commands.find((command) => command.command_type === ICommandType.AddNode)
				?.current_layer,
		).toBe("real-layer");
	});
	test.each(["compact", "balanced", "expanded"] as const)(
		"%s removes generated detours without a catalog template and preserves wires",
		(style) => {
			for (const makeFixture of [fixture, executionFixture]) {
				const input = makeFixture();
				const routed = apply(input, layoutCommands(input).commands);
				const generated = routed.layerNodes.filter((node) => node.auto_reroute);
				expect(generated.length).toBeGreaterThan(0);
				const cleanup = layoutCommands(routed, style);
				const expected = computeFlowLayoutDetailed(
					normalizeAutoReroutes(routed).input,
					style,
				);
				expect(cleanup.result.positions).toEqual(expected.positions);
				for (const node of generated)
					expect(cleanup.result.positions.has(node.id)).toBe(false);
				expect(
					cleanup.commands.filter(
						(command) => command.command_type === ICommandType.RemoveNode,
					),
				).toHaveLength(generated.length);
				const compacted = apply(routed, cleanup.commands);
				expect(compacted.layerNodes.some((node) => node.auto_reroute)).toBe(
					false,
				);
				expect(logicalEdges(compacted)).toEqual(logicalEdges(input));
				expect(
					apply(compacted, layoutCommands(compacted, style).commands),
				).toEqual(compacted);
				const rerouted = apply(compacted, layoutCommands(compacted).commands);
				expect(logicalEdges(rerouted)).toEqual(logicalEdges(input));
				expect(apply(rerouted, layoutCommands(rerouted).commands)).toEqual(
					rerouted,
				);
			}
		},
	);
	test("compact cleanup preserves manual and branched reroutes", () => {
		for (const manual of [true, false]) {
			const routed = apply(fixture(), layoutCommands(fixture()).commands);
			const dot = required(routed.layerNodes.find((node) => node.auto_reroute));
			if (manual) dot.auto_reroute = false;
			else {
				const output = required(
					Object.values(dot.pins).find((pin) => pin.pin_type === "Output"),
				);
				output.connected_to.push("external-input");
			}
			const compacted = apply(
				routed,
				layoutCommands(routed, "compact").commands,
			);
			expect(compacted.layerNodes.some((node) => node.id === dot.id)).toBe(
				true,
			);
			expect(logicalEdges(compacted)).toEqual(logicalEdges(routed));
		}
	});
	test("position-only layouts preserve generated nodes and their topology", () => {
		const routed = apply(fixture(), layoutCommands(fixture()).commands);
		const original = structuredClone(routed);
		for (const style of [
			"compact",
			"balanced",
			"expanded",
			"routed",
		] as const) {
			const positions = computeFlowLayout(routed, style);
			expect([...positions.keys()].sort()).toEqual(
				routed.layerNodes.map((node) => node.id).sort(),
			);
			const preserved = computeFlowLayoutDetailed(
				{ ...routed, preserveAutoReroutes: true },
				style,
			);
			expect(preserved.positions).toEqual(positions);
			expect(preserved.routing).toBeUndefined();
		}
		expect(routed).toEqual(original);
	});
	test("new reroutes require a catalog template", () => {
		const input = fixture();
		const routing = required(
			computeFlowLayoutDetailed(input, "routed").routing,
		);
		expect(() =>
			buildAutoRerouteCommands({
				...routing,
				currentLayer: undefined,
				pinCache: cache(input),
			}),
		).toThrow("The reroute node is missing its routing pins.");
	});
	test("styles without routing remain movement-only on graphs without generated dots", () => {
		for (const style of ["compact", "balanced", "expanded"] as const)
			expect(
				computeFlowLayoutDetailed(fixture(), style).routing,
			).toBeUndefined();
	});
});

describe("shared data reroute commands", () => {
	test("creates one physical trunk, preserves sibling wires and reverses the whole edit", () => {
		const { input, routes } = sharedFixture();
		const source = required(cache(input).get("source:out-0"))[0];
		source.data_type = IVariableType.Struct;
		source.value_type = IValueType.Array;
		source.schema = "shared-records";
		const commands = buildAutoRerouteCommands({
			routes,
			chains: [],
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(input),
		});
		const routed = apply(input, commands);
		const dots = routed.layerNodes.filter((node) => node.auto_reroute);
		expect(dots).toHaveLength(4);
		const chains = chainsFor(routed, routes);
		expect(new Set(chains.map((chain) => chain.nodes[0].id)).size).toBe(1);
		expect(chains[0].nodes[1].id).toBe(chains[1].nodes[1].id);
		const normalized = normalizeAutoReroutes(routed);
		expect(normalized.chains).toHaveLength(routes.length);
		expect(topology(normalized.input)).toEqual(topology(input));
		const links = commands.filter(
			(command) => command.command_type === ICommandType.ConnectPin,
		);
		expect(links).toHaveLength(7);
		expect(
			new Set(links.map((command) => `${command.from_pin}->${command.to_pin}`))
				.size,
		).toBe(links.length);
		expect(
			required(cache(routed).get("source:out-0"))[0].connected_to,
		).toContain("sibling:in-0");
		expect(required(cache(routed).get("source:out-1"))[0].connected_to).toEqual(
			["other:in-0"],
		);
		for (const dot of dots) {
			for (const pin of Object.values(dot.pins)) {
				expect(pin.data_type).toBe(IVariableType.Struct);
				expect(pin.value_type).toBe(IValueType.Array);
				expect(pin.schema).toBe("shared-records");
			}
		}
		expect(topology(apply(routed, inverse(commands)))).toEqual(topology(input));
		expect(
			buildAutoRerouteCommands({
				routes: [...routes].reverse(),
				chains: [...normalized.chains].reverse(),
				currentLayer: undefined,
				pinCache: cache(routed),
			}),
		).toEqual([]);
		const cleanup = buildAutoRerouteCommands({
			routes: routes.map((route) => ({ ...route, waypoints: [] })),
			chains: normalized.chains,
			currentLayer: undefined,
			pinCache: cache(routed),
		});
		expect(
			cleanup.filter(
				(command) => command.command_type === ICommandType.RemoveNode,
			),
		).toHaveLength(4);
		const direct = apply(routed, cleanup);
		expect(topology(direct)).toEqual(topology(input));
		expect(topology(apply(direct, inverse(cleanup)))).toEqual(topology(routed));
	});

	test("merges separate chains and splits a shared tree without reusing one node twice", () => {
		const { input, routes } = sharedFixture();
		const separate = routes.map((route) => ({
			...route,
			waypoints: route.waypoints.map(({ x, y }) => ({ x, y })),
		}));
		const inserted = buildAutoRerouteCommands({
			routes: separate,
			chains: [],
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(input),
		});
		const independent = apply(input, inserted);
		expect(
			independent.layerNodes.filter((node) => node.auto_reroute),
		).toHaveLength(7);
		const merged = buildAutoRerouteCommands({
			routes,
			chains: chainsFor(independent, separate),
			currentLayer: undefined,
			pinCache: cache(independent),
		});
		expect(
			merged.some((command) => command.command_type === ICommandType.AddNode),
		).toBe(false);
		const shared = apply(independent, merged);
		expect(shared.layerNodes.filter((node) => node.auto_reroute)).toHaveLength(
			4,
		);
		const split = buildAutoRerouteCommands({
			routes: separate,
			chains: chainsFor(shared, routes),
			reroute: template(),
			currentLayer: undefined,
			pinCache: cache(shared),
		});
		expect(
			split.filter((command) => command.command_type === ICommandType.AddNode),
		).toHaveLength(3);
		const expanded = apply(shared, split);
		const expandedPaths = chainsFor(expanded, separate).flatMap((chain) =>
			chain.nodes.map((node) => node.id),
		);
		expect(expandedPaths).toHaveLength(7);
		expect(new Set(expandedPaths).size).toBe(7);
		expect(
			buildAutoRerouteCommands({
				routes: separate,
				chains: chainsFor(expanded, separate),
				currentLayer: undefined,
				pinCache: cache(expanded),
			}),
		).toEqual([]);
		expect(topology(apply(expanded, inverse(split)))).toEqual(topology(shared));
	});

	test.each(["unresolved", "omitted", "missing-pin"] as const)(
		"preserves the complete shared tree when one leaf is %s",
		(mode) => {
			const { input, routes } = sharedFixture();
			const routed = apply(
				input,
				buildAutoRerouteCommands({
					routes,
					chains: [],
					reroute: template(),
					currentLayer: undefined,
					pinCache: cache(input),
				}),
			);
			const chains = chainsFor(routed, routes);
			const changed = routes.map((route, index) => ({
				...route,
				unresolved: mode === "unresolved" && index === 0,
				waypoints: route.waypoints.map((point) => ({
					...point,
					y: point.y - 200,
				})),
			}));
			if (mode === "omitted") changed.shift();
			const pins = cache(routed);
			if (mode === "missing-pin") pins.delete(routes[0].toPin);
			expect(
				buildAutoRerouteCommands({
					routes: changed,
					chains,
					currentLayer: undefined,
					pinCache: pins,
				}),
			).toEqual([]);
			const independent: WireRoute = {
				from: "source",
				to: "other",
				fromPin: "source:out-1",
				toPin: "other:in-0",
				waypoints: [{ x: 150, y: 100, shared: "entry" }],
			};
			const commands = buildAutoRerouteCommands({
				routes: [...changed, independent],
				chains,
				reroute: template(),
				currentLayer: undefined,
				pinCache: pins,
			});
			const next = apply(routed, commands);
			expect(next.layerNodes.filter((node) => node.auto_reroute)).toHaveLength(
				5,
			);
			for (const node of routed.layerNodes.filter((node) => node.auto_reroute))
				expect(
					next.layerNodes.find((candidate) => candidate.id === node.id),
				).toEqual(node);
			chainsFor(next, routes);
		},
	);

	test("scopes shared names to the exact source pin", () => {
		const { input, routes } = sharedFixture();
		const independent: WireRoute = {
			from: "source",
			to: "other",
			fromPin: "source:out-1",
			toPin: "other:in-0",
			waypoints: [{ x: 200, y: -80, shared: "entry" }],
		};
		const all = [...routes, independent];
		const routed = apply(
			input,
			buildAutoRerouteCommands({
				routes: all,
				chains: [],
				reroute: template(),
				currentLayer: undefined,
				pinCache: cache(input),
			}),
		);
		expect(routed.layerNodes.filter((node) => node.auto_reroute)).toHaveLength(
			5,
		);
		const paths = chainsFor(routed, all);
		expect(paths[0].nodes[0].id).not.toBe(paths[3].nodes[0].id);
	});

	test("never shares execution reroutes even if waypoint markers repeat", () => {
		const input = executionFixture();
		const route: WireRoute = {
			from: "branch",
			to: "target",
			fromPin: "branch:false",
			toPin: "target:exec-in",
			waypoints: [
				{ x: 250, y: 120, shared: "execution" },
				{ x: 550, y: 120, shared: "execution" },
			],
		};
		const routed = apply(
			input,
			buildAutoRerouteCommands({
				routes: [route],
				chains: [],
				reroute: template(),
				currentLayer: undefined,
				pinCache: cache(input),
			}),
		);
		expect(routed.layerNodes.filter((node) => node.auto_reroute)).toHaveLength(
			2,
		);
		expect(logicalEdges(routed)).toEqual(logicalEdges(input));
	});
});

function required<T>(value: T | null | undefined): T {
	if (value === undefined || value === null)
		throw new Error("Missing reroute graph value");
	return value;
}
