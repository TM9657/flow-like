import { describe, expect, test } from "bun:test";
import { measureNodeBox } from "./measure";
import { type WireRoute, planWireRoutes, sampleWireRoute } from "./route";
import { GraphBuilder } from "./test-fixtures";
import type { AutoLayoutInput, LayoutBox } from "./types";

type Positions = Map<string, [number, number]>;
type Point = { x: number; y: number };

function fourConsumers(
	edgePathType: AutoLayoutInput["edgePathType"] = "default",
) {
	const graph = new GraphBuilder();
	graph.pure("provider", { dataIns: 0 });
	graph.exec("obstacle", { dataIns: 4 });
	const positions: Positions = new Map([
		["provider", [0, 0]],
		["obstacle", [230, 0]],
	]);
	for (let index = 0; index < 4; index++) {
		const id = `consumer-${index}`;
		graph.pure(id, { dataOuts: 0 });
		graph.dataLink("provider", id);
		positions.set(id, [460 + index * 230, 0]);
	}
	return { graph, input: graph.build({ edgePathType }), positions };
}

function physicalDots(routes: WireRoute[]): LayoutBox[] {
	const dots = new Map<string, LayoutBox>();
	for (const route of routes) {
		for (const [index, point] of route.waypoints.entries()) {
			const key = point.shared
				? `${route.fromPin}:${point.shared}`
				: `${route.fromPin}->${route.toPin}:${index}`;
			const box = { x: point.x, y: point.y, width: 16, height: 12 };
			if (dots.has(key)) expect(dots.get(key)).toEqual(box);
			dots.set(key, box);
		}
	}
	return [...dots.values()];
}

function intersectsInterior(a: Point, b: Point, box: LayoutBox): boolean {
	let first = 0;
	let last = 1;
	for (const [start, delta, min, max] of [
		[a.x, b.x - a.x, box.x + 0.01, box.x + box.width - 0.01],
		[a.y, b.y - a.y, box.y + 0.01, box.y + box.height - 0.01],
	]) {
		if (Math.abs(delta) < 0.00001) {
			if (start < min || start > max) return false;
			continue;
		}
		const entry = (min - start) / delta;
		const exit = (max - start) / delta;
		first = Math.max(first, Math.min(entry, exit));
		last = Math.min(last, Math.max(entry, exit));
		if (first > last) return false;
	}
	return true;
}

function expectClear(
	input: AutoLayoutInput,
	positions: Positions,
	routes: WireRoute[],
) {
	const dots = physicalDots(routes);
	const nodes = input.layerNodes.map((node) => {
		const [x, y] = positions.get(node.id) ?? node.coordinates ?? [0, 0];
		return { x, y, ...measureNodeBox(node) };
	});
	const obstacles = [...nodes, ...(input.obstacles ?? [])];
	for (const [index, dot] of dots.entries()) {
		for (const other of [...obstacles, ...dots.slice(0, index)]) {
			expect(
				dot.x < other.x + other.width &&
					dot.x + dot.width > other.x &&
					dot.y < other.y + other.height &&
					dot.y + dot.height > other.y,
			).toBe(false);
		}
	}
	for (const route of routes) {
		expect(route.unresolved).not.toBe(true);
		for (const [a, b] of sampleWireRoute(input, positions, route)) {
			expect(
				[...obstacles, ...dots].some((box) => intersectsInterior(a, b, box)),
			).toBe(false);
		}
	}
}

function sharedByMultipleRoutes(routes: WireRoute[]): boolean {
	const users = new Map<string, Set<string>>();
	for (const route of routes) {
		for (const point of route.waypoints) {
			if (!point.shared) continue;
			const key = `${route.fromPin}:${point.shared}`;
			const targets = users.get(key) ?? new Set<string>();
			targets.add(route.toPin);
			users.set(key, targets);
		}
	}
	return [...users.values()].some((targets) => targets.size >= 2);
}

describe("shared data routes", () => {
	for (const pathType of [
		"default",
		"straight",
		"step",
		"smoothstep",
	] as const) {
		test(`shares a clear trunk across several forward consumers (${pathType})`, () => {
			const { input, positions } = fourConsumers(pathType);
			const routes = planWireRoutes(input, positions);
			expect(routes).toHaveLength(4);
			expect(sharedByMultipleRoutes(routes)).toBe(true);
			expect(physicalDots(routes).length).toBeLessThan(
				routes.reduce((sum, route) => sum + route.waypoints.length, 0),
			);
			expectClear(input, positions, routes);
			const reversed = planWireRoutes(
				{ ...input, layerNodes: [...input.layerNodes].reverse() },
				positions,
			);
			expect(reversed).toEqual(routes);
			expect(planWireRoutes(input, new Map([...positions].reverse()))).toEqual(
				routes,
			);
			const prefixes = new Map<string, WireRoute["waypoints"]>();
			for (const route of routes) {
				for (const [index, point] of route.waypoints.entries()) {
					if (!point.shared) continue;
					const key = `${route.fromPin}:${point.shared}`;
					const prefix = route.waypoints.slice(0, index + 1);
					const previous = prefixes.get(key);
					if (previous) expect(prefix).toEqual(previous);
					prefixes.set(key, prefix);
				}
			}
		});
	}
	test("keeps a clear nearby consumer directly connected", () => {
		const { graph, positions } = fourConsumers();
		graph.nodes.delete("obstacle");
		positions.delete("obstacle");
		graph.pure("nearby", { dataOuts: 0 });
		graph.dataLink("provider", "nearby");
		positions.set("nearby", [230, 0]);
		const input = graph.build();
		const routes = planWireRoutes(input, positions);
		expect(routes.find((route) => route.to === "nearby")?.waypoints).toEqual(
			[],
		);
		expect(sharedByMultipleRoutes(routes)).toBe(true);
		expectClear(input, positions, routes);
	});

	test("keeps a shared trunk out of an independent execution branch", () => {
		const graph = new GraphBuilder();
		const positions: Positions = new Map();
		for (const [branch, y] of [
			["upper", 0],
			["lower", 210],
		] as const) {
			for (let index = 0; index < 5; index++) {
				const id = `${branch}-${index}`;
				graph.exec(id, {
					execIn: index !== 0,
					start: index === 0,
					dataIns: 1,
					dataOuts: index === 0 ? 1 : 0,
				});
				positions.set(id, [index * 230, y]);
				if (index > 0) graph.execLink(`${branch}-${index - 1}`, id);
				if (branch === "upper" && index >= 2) graph.dataLink("upper-0", id);
			}
		}
		const input = graph.build();
		const routes = planWireRoutes(input, positions);
		expect(sharedByMultipleRoutes(routes)).toBe(true);
		for (const route of routes.filter((route) =>
			route.waypoints.some((point) => point.shared),
		)) {
			for (const [a, b] of sampleWireRoute(input, positions, route)) {
				expect(Math.max(a.y, b.y)).toBeLessThan(160);
			}
		}
		expectClear(input, positions, routes);
	});

	test("keeps consumers outside a selected layout out of shared routes", () => {
		const { input, positions } = fourConsumers();
		input.only = new Set(["provider", "consumer-0", "consumer-1"]);
		const routes = planWireRoutes(input, positions);
		expect(routes.map((route) => route.to).sort()).toEqual([
			"consumer-0",
			"consumer-1",
		]);
		expectClear(input, positions, routes);
	});

	test("never shares a waypoint between distinct source pins", () => {
		const { graph, positions } = fourConsumers();
		graph.pure("provider", { dataIns: 0, dataOuts: 2 });
		for (let index = 0; index < 4; index++) {
			graph.dataLink("provider", `consumer-${index}`);
			const id = `other-${index}`;
			graph.pure(id, { dataOuts: 0 });
			graph.dataLink("provider", id, 1);
			positions.set(id, [460 + index * 230, 240]);
		}
		const input = graph.build();
		const routes = planWireRoutes(input, positions);
		const owners = new Map<string, string>();
		for (const route of routes) {
			for (const point of route.waypoints) {
				if (!point.shared) continue;
				const existing = owners.get(point.shared);
				if (existing) expect(existing).toBe(route.fromPin);
				owners.set(point.shared, route.fromPin);
			}
		}
		expect(sharedByMultipleRoutes(routes)).toBe(true);
		expectClear(input, positions, routes);
	});

	test("leaves a single connection and backward connections unshared", () => {
		const graph = new GraphBuilder();
		graph.pure("provider", { dataIns: 0 });
		graph.pure("left");
		graph.pure("right");
		graph.dataLink("provider", "left");
		graph.dataLink("provider", "right");
		const positions: Positions = new Map([
			["provider", [460, 0]],
			["left", [0, 0]],
			["right", [920, 0]],
		]);
		const input = graph.build();
		const routes = planWireRoutes(input, positions);
		expect(routes).toHaveLength(2);
		expect(
			routes.flatMap((route) => route.waypoints).some((p) => p.shared),
		).toBe(false);
	});

	test("does not turn execution fan-out into a shared data trunk", () => {
		const graph = new GraphBuilder();
		graph.exec("source", { execIn: false, start: true });
		graph.exec("obstacle", { dataIns: 4 });
		const positions: Positions = new Map([
			["source", [0, 0]],
			["obstacle", [230, 0]],
		]);
		for (let index = 0; index < 3; index++) {
			graph.exec(`target-${index}`);
			graph.execLink("source", `target-${index}`);
			positions.set(`target-${index}`, [460 + index * 230, 0]);
		}
		const routes = planWireRoutes(graph.build(), positions);
		expect(
			routes.flatMap((route) => route.waypoints).some((p) => p.shared),
		).toBe(false);
	});
});
