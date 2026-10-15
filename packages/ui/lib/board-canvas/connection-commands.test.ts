import { describe, expect, test } from "bun:test";
import type { Connection, Edge } from "@xyflow/react";
import { type ILayer, ILayerType } from "../schema/flow/board";
import {
	ICommandType,
	type IGenericCommand,
} from "../schema/flow/board/commands/generic-command";
import type { INode } from "../schema/flow/node";
import {
	type IPin,
	IPinType,
	IValueType,
	IVariableType,
} from "../schema/flow/pin";
import {
	buildConnectionCommands,
	buildDisconnectionCommands,
} from "./connection-commands";

function pin(id: string, pin_type: IPinType, data_type: IVariableType): IPin {
	return {
		id,
		name: id,
		friendly_name: id,
		description: "",
		pin_type,
		data_type,
		value_type: IValueType.Normal,
		index: 0,
		connected_to: [],
		depends_on: [],
	};
}

function fixture(kind = IVariableType.String, links: [string, string][] = []) {
	const boardNodes: Record<string, INode> = {};
	const pinCache = new Map<string, [IPin, INode | ILayer, boolean]>();
	for (const id of ["a", "b", "c", "d", "e"]) {
		const pins = [
			pin(`${id}.out`, IPinType.Output, kind),
			pin(`${id}.in`, IPinType.Input, kind),
		];
		const node: INode = {
			id,
			name: id,
			friendly_name: id,
			description: "",
			category: "test",
			pins: Object.fromEntries(pins.map((p) => [p.id, p])),
			fn_refs: {
				fn_refs: [],
				can_reference_fns: true,
				can_be_referenced_by_fns: true,
				custom_flag: "preserve",
			},
		};
		boardNodes[id] = node;
		for (const p of pins) pinCache.set(p.id, [p, node, true]);
	}
	for (const [from, to] of links) {
		boardNodes[from].pins[`${from}.out`].connected_to.push(`${to}.in`);
		boardNodes[to].pins[`${to}.in`].depends_on.push(`${from}.out`);
	}
	return { boardNodes, pinCache, refs: {} as Record<string, string> };
}

const connection = (from: string, to: string): Connection => ({
	source: from,
	target: to,
	sourceHandle: `${from}.out`,
	targetHandle: `${to}.in`,
});
const edge = (from: string, to: string): Edge => ({
	...connection(from, to),
	id: `${from}-${to}`,
});
const reference = (from: string, to: string): Connection => ({
	source: from,
	target: to,
	sourceHandle: `ref_out_${from}`,
	targetHandle: `ref_in_${to}`,
});
const referenceEdge = (from: string, to: string): Edge => ({
	...reference(from, to),
	id: `reference-${from}-${to}`,
});

function requireCommands(
	commands: IGenericCommand[] | undefined,
): IGenericCommand[] {
	if (!commands) throw new Error("Expected a valid transaction");
	return commands;
}

function freeze<T>(value: T): T {
	if (value && typeof value === "object") {
		for (const child of Object.values(value)) freeze(child);
		Object.freeze(value);
	}
	return value;
}

/** Replay the relevant backend effects and the inverse commands recorded in its receipts. */
function apply(nodes: Record<string, INode>, commands: IGenericCommand[]) {
	const receipts = commands.map((command) => {
		if (command.command_type === ICommandType.UpdateNode && command.node) {
			const old_node = structuredClone(nodes[command.node.id]);
			nodes[command.node.id] = structuredClone(command.node as INode);
			return { ...command, old_node };
		}
		const { from_node, from_pin, to_node, to_pin } = command;
		if (!from_node || !from_pin || !to_node || !to_pin)
			throw new Error("Missing wire endpoint");
		const output = nodes[from_node].pins[from_pin];
		const input = nodes[to_node].pins[to_pin];
		if (command.command_type === ICommandType.DisconnectPin) {
			output.connected_to = output.connected_to.filter((id) => id !== input.id);
			input.depends_on = input.depends_on.filter((id) => id !== output.id);
		} else if (command.command_type === ICommandType.ConnectPin) {
			if (output.data_type === IVariableType.Execution) {
				output.connected_to = [];
				for (const node of Object.values(nodes)) {
					for (const pin of Object.values(node.pins))
						pin.depends_on = pin.depends_on.filter((id) => id !== output.id);
				}
			} else input.depends_on = [];
			if (!output.connected_to.includes(input.id))
				output.connected_to.push(input.id);
			if (!input.depends_on.includes(output.id))
				input.depends_on.push(output.id);
		} else throw new Error(`Unexpected command ${command.command_type}`);
		return command;
	});
	return receipts;
}

function undo(nodes: Record<string, INode>, receipts: IGenericCommand[]) {
	for (const command of receipts.toReversed()) {
		if (command.command_type === ICommandType.UpdateNode && command.old_node) {
			nodes[command.old_node.id] = structuredClone(command.old_node as INode);
		} else
			apply(nodes, [
				{
					...command,
					command_type:
						command.command_type === ICommandType.ConnectPin
							? ICommandType.DisconnectPin
							: ICommandType.ConnectPin,
				},
			]);
	}
}

function connections(nodes: Record<string, INode>) {
	return Object.values(nodes)
		.flatMap((node) =>
			Object.values(node.pins).flatMap((p) =>
				p.connected_to.map((target) => `${p.id}->${target}`),
			),
		)
		.sort();
}

function assertRoundTrip(
	source: Record<string, INode>,
	commands: IGenericCommand[],
	expected: string[],
) {
	const state = structuredClone(source);
	const receipts = apply(state, commands);
	expect(connections(state)).toEqual(expected.toSorted());
	undo(state, receipts);
	const original = structuredClone(source);
	for (const nodes of [state, original]) {
		for (const node of Object.values(nodes)) {
			for (const p of Object.values(node.pins)) {
				p.connected_to.sort();
				p.depends_on.sort();
			}
		}
	}
	expect(state).toEqual(original);
}

describe("connection transactions", () => {
	test("replacing a data input records its displaced wire and preserves output fan-out on undo", () => {
		const data = fixture(IVariableType.String, [
			["a", "c"],
			["a", "d"],
		]);
		freeze(data.boardNodes);
		const commands = requireCommands(
			buildConnectionCommands({ ...data, connection: connection("b", "c") }),
		);
		expect(commands.map((c) => c.command_type)).toEqual([
			ICommandType.DisconnectPin,
			ICommandType.ConnectPin,
		]);
		expect(commands[0]).toMatchObject({ from_node: "a", to_node: "c" });
		assertRoundTrip(data.boardNodes, commands, ["b.out->c.in", "a.out->d.in"]);
	});

	test("replacing an execution output preserves other incoming execution wires", () => {
		const data = fixture(IVariableType.Execution, [
			["a", "c"],
			["b", "d"],
		]);
		const commands = requireCommands(
			buildConnectionCommands({ ...data, connection: connection("a", "d") }),
		);
		expect(commands).toHaveLength(2);
		assertRoundTrip(data.boardNodes, commands, ["a.out->d.in", "b.out->d.in"]);
	});

	test("reconnection restores both the old wire and an occupied destination on undo", () => {
		const data = fixture(IVariableType.String, [
			["a", "c"],
			["b", "d"],
			["e", "b"],
		]);
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				oldEdge: edge("a", "c"),
				connection: connection("e", "d"),
			}),
		);
		expect(
			commands.map((c) => [c.command_type, c.from_node, c.to_node]),
		).toEqual([
			[ICommandType.DisconnectPin, "a", "c"],
			[ICommandType.DisconnectPin, "b", "d"],
			[ICommandType.ConnectPin, "e", "d"],
		]);
		assertRoundTrip(data.boardNodes, commands, ["e.out->b.in", "e.out->d.in"]);
	});

	test("reconnecting an occupied execution source disconnects the old wire only once", () => {
		const data = fixture(IVariableType.Execution, [["a", "c"]]);
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				oldEdge: edge("a", "c"),
				connection: connection("a", "d"),
			}),
		);
		expect(commands).toHaveLength(2);
		assertRoundTrip(data.boardNodes, commands, ["a.out->d.in"]);
	});

	test("duplicate and unchanged connections do not create undo entries", () => {
		const data = fixture(IVariableType.String, [["a", "c"]]);
		expect(
			buildConnectionCommands({ ...data, connection: connection("a", "c") }),
		).toEqual([]);
		expect(
			buildConnectionCommands({
				...data,
				oldEdge: edge("a", "c"),
				connection: connection("a", "c"),
			}),
		).toEqual([]);
	});

	test("reconnecting onto an existing wire removes only the previous wire", () => {
		const data = fixture(IVariableType.String, [
			["a", "c"],
			["b", "d"],
		]);
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				oldEdge: edge("a", "c"),
				connection: connection("b", "d"),
			}),
		);
		expect(commands).toHaveLength(1);
		assertRoundTrip(data.boardNodes, commands, ["b.out->d.in"]);
	});

	test("backward drags and visual proxies use the canonical cache owners", () => {
		const data = fixture();
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				connection: {
					source: "collapsed-view",
					target: "layer-input",
					sourceHandle: "c.in",
					targetHandle: "a.out",
				},
			}),
		);
		expect(commands[0]).toMatchObject({
			from_node: "a",
			from_pin: "a.out",
			to_node: "c",
			to_pin: "c.in",
		});
	});

	test("inner-layer output pins keep their canonical owner and visible direction", () => {
		const data = fixture();
		const boundaryPin = pin("layer.in", IPinType.Input, IVariableType.String);
		const layer: ILayer = {
			id: "layer",
			name: "Layer",
			type: ILayerType.Collapsed,
			coordinates: [0, 0],
			pins: { [boundaryPin.id]: boundaryPin },
			nodes: {},
			comments: {},
			variables: {},
		};
		data.pinCache.set(boundaryPin.id, [
			{ ...boundaryPin, pin_type: IPinType.Output },
			layer,
			true,
		]);
		freeze(layer);
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				connection: {
					source: "layer-input",
					sourceHandle: boundaryPin.id,
					target: "b",
					targetHandle: "b.in",
				},
			}),
		);
		expect(commands).toHaveLength(1);
		expect(commands[0]).toMatchObject({
			from_node: "layer",
			from_pin: boundaryPin.id,
			to_node: "b",
			to_pin: "b.in",
		});
		expect(boundaryPin.pin_type).toBe(IPinType.Input);
	});

	test("a visual proxy cannot bypass same-owner or same-pin rejection", () => {
		const data = fixture();
		expect(
			buildConnectionCommands({
				...data,
				connection: { ...connection("a", "a"), target: "a-proxy" },
			}),
		).toBeUndefined();
		expect(
			buildConnectionCommands({
				...data,
				connection: { ...connection("a", "b"), targetHandle: "a.out" },
			}),
		).toBeUndefined();
	});

	test("schema validation resolves refs before constructing a transaction", () => {
		const data = fixture(IVariableType.Struct);
		data.boardNodes.a.pins["a.out"].schema = "output";
		data.boardNodes.b.pins["b.in"].schema = "input";
		data.refs.output =
			'{"type":"object","properties":{"name":{"type":"string"}}}';
		data.refs.input =
			'{"type":"object","properties":{"count":{"type":"number"}}}';
		expect(
			buildConnectionCommands({ ...data, connection: connection("a", "b") }),
		).toBeUndefined();
	});

	test("missing displaced endpoints cannot produce an undo-incomplete batch", () => {
		const data = fixture(IVariableType.String, [["a", "c"]]);
		data.pinCache.delete("a.out");
		expect(
			buildConnectionCommands({ ...data, connection: connection("b", "c") }),
		).toBeUndefined();
	});

	test("stale reconnects and incomplete proposals are rejected", () => {
		const data = fixture();
		expect(
			buildConnectionCommands({
				...data,
				oldEdge: edge("a", "b"),
				connection: connection("c", "d"),
			}),
		).toBeUndefined();
		expect(
			buildConnectionCommands({
				...data,
				connection: { ...connection("a", "b"), sourceHandle: null },
			}),
		).toBeUndefined();
	});
});

describe("function reference transactions", () => {
	test("adds a reference without mutating shared node state or flags", () => {
		const data = fixture();
		freeze(data.boardNodes);
		const commands = requireCommands(
			buildConnectionCommands({ ...data, connection: reference("a", "b") }),
		);
		expect(commands[0].node?.fn_refs).toEqual({
			...data.boardNodes.a.fn_refs,
			fn_refs: ["b"],
		});
		assertRoundTrip(data.boardNodes, commands, []);
	});

	test("target reconnection emits one combined update and preserves sibling references", () => {
		const data = fixture();
		data.boardNodes.a.fn_refs = {
			can_reference_fns: true,
			can_be_referenced_by_fns: false,
			fn_refs: ["b", "c"],
		};
		freeze(data.boardNodes);
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				oldEdge: referenceEdge("a", "b"),
				connection: reference("a", "d"),
			}),
		);
		expect(commands).toHaveLength(1);
		expect(commands[0].node?.fn_refs).toEqual({
			...data.boardNodes.a.fn_refs,
			fn_refs: ["c", "d"],
		});
		assertRoundTrip(data.boardNodes, commands, []);
	});

	test("source reconnection updates both owners and restores both on undo", () => {
		const data = fixture();
		data.boardNodes.a.fn_refs = {
			can_reference_fns: true,
			can_be_referenced_by_fns: false,
			fn_refs: ["b", "c"],
		};
		data.boardNodes.d.fn_refs = {
			can_reference_fns: true,
			can_be_referenced_by_fns: true,
			fn_refs: ["e"],
		};
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				oldEdge: referenceEdge("a", "b"),
				connection: {
					source: "b",
					target: "d",
					sourceHandle: "ref_in_b",
					targetHandle: "ref_out_d",
				},
			}),
		);
		expect(commands.map((c) => [c.node?.id, c.node?.fn_refs?.fn_refs])).toEqual(
			[
				["a", ["c"]],
				["d", ["e", "b"]],
			],
		);
		assertRoundTrip(data.boardNodes, commands, []);
	});

	test("duplicate refs are no-ops and self/missing/mixed endpoints are rejected", () => {
		const data = fixture();
		data.boardNodes.a.fn_refs = {
			can_reference_fns: true,
			can_be_referenced_by_fns: true,
			fn_refs: ["b"],
		};
		expect(
			buildConnectionCommands({ ...data, connection: reference("a", "b") }),
		).toEqual([]);
		expect(
			buildConnectionCommands({ ...data, connection: reference("a", "a") }),
		).toBeUndefined();
		expect(
			buildConnectionCommands({
				...data,
				connection: reference("a", "missing"),
			}),
		).toBeUndefined();
		expect(
			buildConnectionCommands({
				...data,
				connection: { ...reference("a", "b"), targetHandle: "b.in" },
			}),
		).toBeUndefined();
	});

	test("reconnecting onto an existing reference removes only the old target", () => {
		const data = fixture();
		if (!data.boardNodes.a.fn_refs) throw new Error("Missing fixture refs");
		data.boardNodes.a.fn_refs.fn_refs = ["b", "c"];
		const commands = requireCommands(
			buildConnectionCommands({
				...data,
				oldEdge: referenceEdge("a", "b"),
				connection: reference("a", "c"),
			}),
		);
		expect(commands).toHaveLength(1);
		expect(commands[0].node?.fn_refs?.fn_refs).toEqual(["c"]);
		assertRoundTrip(data.boardNodes, commands, []);
	});
});

describe("disconnection transactions", () => {
	test("disconnects an existing pin wire and restores it on undo", () => {
		const data = fixture(IVariableType.Execution, [["a", "b"]]);
		const commands = requireCommands(
			buildDisconnectionCommands({ ...data, edge: edge("a", "b") }),
		);
		assertRoundTrip(data.boardNodes, commands, []);
		expect(
			buildDisconnectionCommands({ ...data, edge: edge("c", "d") }),
		).toEqual([]);
	});

	test("disconnects a function reference while preserving sibling refs and flags", () => {
		const data = fixture();
		data.boardNodes.a.fn_refs = {
			can_reference_fns: true,
			can_be_referenced_by_fns: false,
			fn_refs: ["b", "c"],
		};
		freeze(data.boardNodes);
		const commands = requireCommands(
			buildDisconnectionCommands({ ...data, edge: referenceEdge("a", "b") }),
		);
		expect(commands[0].node?.fn_refs).toEqual({
			...data.boardNodes.a.fn_refs,
			fn_refs: ["c"],
		});
		assertRoundTrip(data.boardNodes, commands, []);
	});
});
