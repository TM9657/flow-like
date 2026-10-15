import { describe, expect, test } from "bun:test";
import type { INode } from "../schema/flow/node";
import { IVariableType } from "../schema/flow/node";
import { normalizeAutoReroutes } from "./normalize-reroutes";
import { GraphBuilder } from "./test-fixtures";
import type { AutoLayoutInput } from "./types";

function reroute(graph: GraphBuilder, id: string): INode {
	const node = graph.pure(id);
	node.name = "reroute";
	node.auto_reroute = true;
	node.pins[`${id}:in-0`].name = "route_in";
	node.pins[`${id}:out-0`].name = "route_out";
	return node;
}

function mirrorConnections(input: AutoLayoutInput): void {
	const pins = new Map(
		input.layerNodes.flatMap((node) =>
			Object.values(node.pins).map((pin) => [pin.id, pin] as const),
		),
	);
	for (const pin of pins.values()) {
		for (const target of pin.connected_to)
			pins.get(target)?.depends_on.push(pin.id);
	}
}

function tree(): AutoLayoutInput {
	const graph = new GraphBuilder();
	graph.pure("source", { dataIns: 0 });
	for (const id of ["a", "b", "c", "direct"]) graph.pure(id);
	for (const id of ["trunk", "junction", "leaf-a", "leaf-c"])
		reroute(graph, id);
	graph.dataLink("source", "trunk");
	graph.dataLink("source", "direct");
	graph.dataLink("trunk", "junction");
	graph.dataLink("junction", "leaf-a");
	graph.dataLink("junction", "b");
	graph.dataLink("junction", "leaf-c");
	graph.dataLink("leaf-a", "a");
	graph.dataLink("leaf-c", "c");
	const input = graph.build();
	mirrorConnections(input);
	return input;
}

function node(input: AutoLayoutInput, id: string): INode {
	const found = input.layerNodes.find((value) => value.id === id);
	if (!found) throw new Error(`Missing fixture node ${id}`);
	return found;
}

function expectPreserved(input: AutoLayoutInput): void {
	const before = structuredClone(input);
	const normalized = normalizeAutoReroutes(input);
	expect(normalized.chains).toEqual([]);
	expect(normalized.input).toBe(input);
	expect(input).toEqual(before);
}

describe("generated reroute tree normalization", () => {
	test("flattens a complete tree and reports each terminal's shared path", () => {
		const input = tree();
		const before = structuredClone(input);
		const normalized = normalizeAutoReroutes(input);
		expect(normalized.chains).toHaveLength(3);
		expect(
			Object.fromEntries(
				normalized.chains.map((chain) => [
					chain.to,
					chain.nodes.map((value) => value.id),
				]),
			),
		).toEqual({
			a: ["trunk", "junction", "leaf-a"],
			b: ["trunk", "junction"],
			c: ["trunk", "junction", "leaf-c"],
		});
		for (const chain of normalized.chains) {
			expect(chain.from).toBe("source");
			expect(chain.fromPin).toBe("source:out-0");
			expect(chain.toPin).toBe(`${chain.to}:in-0`);
			expect(chain.nodes[0]).toBe(node(input, "trunk"));
			expect(
				node(normalized.input, chain.to).pins[chain.toPin].depends_on,
			).toEqual(["source:out-0"]);
		}
		expect(
			normalized.input.layerNodes.some((value) => value.auto_reroute),
		).toBe(false);
		expect(
			node(normalized.input, "source").pins["source:out-0"].connected_to.sort(),
		).toEqual(["a:in-0", "b:in-0", "c:in-0", "direct:in-0"]);
		expect(input).toEqual(before);
		expect(normalizeAutoReroutes(normalized.input).chains).toEqual([]);
		expect(
			normalizeAutoReroutes({
				...input,
				layerNodes: [...input.layerNodes].reverse(),
			}).chains,
		).toEqual(normalized.chains);
	});

	test("requires every terminal endpoint to be selected before removing a shared trunk", () => {
		const input = tree();
		for (const only of [
			new Set(["source", "a", "b"]),
			new Set(["a", "b", "c"]),
			new Set(["trunk", "junction", "leaf-a", "leaf-c", "a", "b", "c"]),
		])
			expectPreserved({ ...input, only });
		const normalized = normalizeAutoReroutes({
			...input,
			only: new Set(["source", "a", "b", "c", "trunk"]),
			obstacles: [
				{ id: "trunk", x: 0, y: 0, width: 16, height: 12 },
				{ id: "leaf-c", x: 20, y: 0, width: 16, height: 12 },
				{ id: "direct", x: 40, y: 0, width: 150, height: 43 },
			],
		});
		expect(normalized.chains).toHaveLength(3);
		expect(normalized.input.only).toEqual(new Set(["source", "a", "b", "c"]));
		expect(normalized.input.obstacles?.map((box) => box.id)).toEqual([
			"direct",
		]);
	});

	test("recognizes connections recorded only on their input pins", () => {
		const input = tree();
		for (const value of input.layerNodes)
			for (const pin of Object.values(value.pins)) pin.connected_to = [];
		const normalized = normalizeAutoReroutes(input);
		expect(normalized.chains).toHaveLength(3);
		expect(
			node(normalized.input, "source").pins["source:out-0"].connected_to.sort(),
		).toEqual(["a:in-0", "b:in-0", "c:in-0"]);
	});

	test.each([
		[
			"unknown branch",
			(input: AutoLayoutInput) => {
				node(input, "junction").pins["junction:out-0"].connected_to.push(
					"external",
				);
			},
		],
		[
			"unknown source",
			(input: AutoLayoutInput) => {
				node(input, "source").pins["source:out-0"].connected_to = [
					"direct:in-0",
				];
				node(input, "trunk").pins["trunk:in-0"].depends_on = ["external"];
			},
		],
		[
			"WASM descendant",
			(input: AutoLayoutInput) => {
				node(input, "leaf-a").wasm = { package_id: "custom", permissions: [] };
			},
		],
		[
			"referenced descendant",
			(input: AutoLayoutInput) => {
				node(input, "direct").fn_refs = {
					can_be_referenced_by_fns: false,
					can_reference_fns: true,
					fn_refs: ["leaf-a"],
				};
			},
		],
		[
			"disconnected leaf",
			(input: AutoLayoutInput) => {
				node(input, "leaf-a").pins["leaf-a:out-0"].connected_to = [];
				node(input, "a").pins["a:in-0"].depends_on = [];
			},
		],
		[
			"wrong target pin direction",
			(input: AutoLayoutInput) => {
				node(input, "junction").pins["junction:out-0"].connected_to.push(
					"a:out-0",
				);
			},
		],
		[
			"wrong reroute pin direction",
			(input: AutoLayoutInput) => {
				node(input, "junction").pins["junction:out-0"].connected_to.push(
					"leaf-a:out-0",
				);
			},
		],
		[
			"mixed execution pins",
			(input: AutoLayoutInput) => {
				node(input, "leaf-a").pins["leaf-a:out-0"].data_type =
					IVariableType.Execution;
			},
		],
		[
			"data fan-in",
			(input: AutoLayoutInput) => {
				node(input, "direct").pins["direct:out-0"].connected_to.push(
					"leaf-a:in-0",
				);
			},
		],
		[
			"terminal fan-in",
			(input: AutoLayoutInput) => {
				node(input, "leaf-c").pins["leaf-c:out-0"].connected_to.push("a:in-0");
			},
		],
		[
			"duplicate direct connection",
			(input: AutoLayoutInput) => {
				node(input, "source").pins["source:out-0"].connected_to.push("a:in-0");
			},
		],
	] as const)("preserves the whole generated tree with %s", (_, change) => {
		const input = tree();
		change(input);
		expectPreserved(input);
	});

	test("preserves execution branching and execution fan-in", () => {
		const input = tree();
		for (const value of input.layerNodes)
			for (const pin of Object.values(value.pins))
				pin.data_type = IVariableType.Execution;
		expectPreserved(input);
		const graph = new GraphBuilder();
		graph.exec("a");
		graph.exec("b");
		graph.exec("target");
		const merge = reroute(graph, "merge");
		for (const pin of Object.values(merge.pins))
			pin.data_type = IVariableType.Execution;
		graph.connect("a:exec-out-0", "merge:in-0");
		graph.connect("b:exec-out-0", "merge:in-0");
		graph.connect("merge:out-0", "target:exec-in");
		expectPreserved(graph.build());
	});

	test("normalizes separate execution chains from one output without sharing their paths", () => {
		const graph = new GraphBuilder();
		graph.exec("source");
		for (const id of ["a", "b"]) {
			graph.exec(`target-${id}`);
			const route = reroute(graph, id);
			for (const pin of Object.values(route.pins))
				pin.data_type = IVariableType.Execution;
			graph.connect("source:exec-out-0", `${id}:in-0`);
			graph.connect(`${id}:out-0`, `target-${id}:exec-in`);
		}
		const input = graph.build();
		mirrorConnections(input);
		const normalized = normalizeAutoReroutes(input);
		expect(normalized.chains).toHaveLength(2);
		expect(
			normalized.chains.map((chain) => chain.nodes.map((value) => value.id)),
		).toEqual([["a"], ["b"]]);
		expect(
			node(normalized.input, "source").pins[
				"source:exec-out-0"
			].connected_to.sort(),
		).toEqual(["target-a:exec-in", "target-b:exec-in"]);
		for (const id of ["a", "b"])
			expect(
				node(normalized.input, `target-${id}`).pins[`target-${id}:exec-in`]
					.depends_on,
			).toEqual(["source:exec-out-0"]);
	});

	test("preserves generated cycles", () => {
		const graph = new GraphBuilder();
		reroute(graph, "a");
		reroute(graph, "b");
		graph.pure("target");
		graph.dataLink("a", "b");
		graph.dataLink("b", "a");
		graph.dataLink("b", "target");
		expectPreserved(graph.build());
	});

	test("retains manual boundaries while normalizing their generated prefixes and suffixes", () => {
		const input = tree();
		node(input, "junction").auto_reroute = false;
		const normalized = normalizeAutoReroutes(input);
		expect(
			normalized.chains.map((chain) => [chain.from, chain.to]).sort(),
		).toEqual([
			["junction", "a"],
			["junction", "c"],
			["source", "junction"],
		]);
		expect(node(normalized.input, "junction").auto_reroute).toBe(false);
		expect(
			node(normalized.input, "junction").pins[
				"junction:out-0"
			].connected_to.sort(),
		).toEqual(["a:in-0", "b:in-0", "c:in-0"]);
		expect(node(normalized.input, "a").pins["a:in-0"].depends_on).toEqual([
			"junction:out-0",
		]);
	});

	test("does not absorb a manual suffix or its unselected generated branch", () => {
		const input = tree();
		node(input, "junction").auto_reroute = false;
		const normalized = normalizeAutoReroutes({
			...input,
			only: new Set(["source", "junction", "a"]),
		});
		expect(
			normalized.chains.map((chain) => [chain.from, chain.to]).sort(),
		).toEqual([
			["junction", "a"],
			["source", "junction"],
		]);
		expect(node(normalized.input, "leaf-c")).toEqual(node(input, "leaf-c"));
		expect(
			node(normalized.input, "junction").pins["junction:out-0"].connected_to,
		).toContain("leaf-c:in-0");
	});

	test("preserves a whole tree when one terminal belongs to its source's open layer", () => {
		const input = tree();
		node(input, "source").id = "fn-input";
		node(input, "c").id = "fn-return";
		input.currentLayer = "fn";
		expectPreserved(input);
	});

	test("preserves a whole tree when one terminal returns to its source node", () => {
		const input = tree();
		const source = node(input, "source");
		const target = node(input, "c");
		source.pins["c:in-0"] = target.pins["c:in-0"];
		input.layerNodes = input.layerNodes.filter((value) => value !== target);
		expectPreserved(input);
	});

	test("rewrites a selected layer entity terminal without mutating its stored pins", () => {
		const input = tree();
		const graph = new GraphBuilder();
		graph.layer("function");
		const entityInput = node(input, "c").pins["c:in-0"];
		graph.boardLayers.function.pins = { [entityInput.id]: entityInput };
		input.layerNodes = input.layerNodes.filter((value) => value.id !== "c");
		input.layerEntities = graph.entities;
		input.boardLayers = graph.boardLayers;
		input.only = new Set(["source", "a", "b", "function"]);
		const before = structuredClone(input);
		const normalized = normalizeAutoReroutes(input);
		expect(normalized.chains).toHaveLength(3);
		expect(
			normalized.chains.find((chain) => chain.to === "function")?.toPin,
		).toBe("c:in-0");
		expect(
			normalized.input.boardLayers?.function.pins["c:in-0"].depends_on,
		).toEqual(["source:out-0"]);
		expect(input).toEqual(before);
	});
});
