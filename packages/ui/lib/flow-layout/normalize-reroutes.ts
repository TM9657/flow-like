import type { INode, IPin } from "../schema/flow/node";
import type { AutoLayoutInput } from "./types";
export interface AutoRerouteChain {
	from: string;
	to: string;
	fromPin: string;
	toPin: string;
	/** Original path in wire order. Shared trunks occur in multiple paths. */
	nodes: INode[];
}
/** Treat complete generated data trees and execution chains as wire geometry. */
export function normalizeAutoReroutes(input: AutoLayoutInput): {
	input: AutoLayoutInput;
	chains: AutoRerouteChain[];
} {
	const owners = new Map<string, string>();
	const pins = new Map<string, IPin>();
	const addPins = (id: string, values: Record<string, IPin>) => {
		for (const pin of Object.values(values)) {
			owners.set(pin.id, id);
			pins.set(pin.id, pin);
		}
	};
	for (const node of input.layerNodes) addPins(node.id, node.pins);
	for (const entity of input.layerEntities) {
		addPins(entity.id, input.boardLayers?.[entity.id]?.pins ?? {});
	}
	const incoming = new Map<string, Set<string>>();
	const outgoing = new Map<string, Set<string>>();
	const link = (from: string, to: string) => {
		const targets = outgoing.get(from) ?? new Set<string>();
		targets.add(to);
		outgoing.set(from, targets);
		const sources = incoming.get(to) ?? new Set<string>();
		sources.add(from);
		incoming.set(to, sources);
	};
	for (const pin of pins.values()) {
		if (pin.pin_type === "Output") {
			for (const target of pin.connected_to) link(pin.id, target);
		} else {
			for (const source of pin.depends_on) link(source, pin.id);
		}
	}
	const referenced = new Set(
		input.layerNodes.flatMap((node) => node.fn_refs?.fn_refs ?? []),
	);
	const generated = new Map(
		input.layerNodes
			.filter((node) => node.name === "reroute" && node.auto_reroute === true)
			.map((node) => [node.id, node]),
	);
	const neighbors = new Map<string, Set<string>>();
	for (const [from, targets] of outgoing) {
		const source = owners.get(from);
		if (!source || !generated.has(source)) continue;
		for (const to of targets) {
			const target = owners.get(to);
			if (!target || !generated.has(target)) continue;
			const sourceNeighbors = neighbors.get(source) ?? new Set<string>();
			sourceNeighbors.add(target);
			neighbors.set(source, sourceNeighbors);
			const targetNeighbors = neighbors.get(target) ?? new Set<string>();
			targetNeighbors.add(source);
			neighbors.set(target, targetNeighbors);
		}
	}
	const candidates = new Map<
		string,
		{
			node: INode;
			input: IPin;
			output: IPin;
		}
	>();
	for (const node of generated.values()) {
		if (node.wasm || referenced.has(node.id)) continue;
		const values = Object.values(node.pins);
		const routeIn = values.find(
			(pin) => pin.name === "route_in" && pin.pin_type === "Input",
		);
		const routeOut = values.find(
			(pin) => pin.name === "route_out" && pin.pin_type === "Output",
		);
		if (values.length !== 2 || !routeIn || !routeOut) continue;
		if (routeIn.connected_to.length || routeOut.depends_on.length) continue;
		const isExecution = routeIn.data_type === "Execution";
		if (
			(incoming.get(routeIn.id)?.size ?? 0) !== 1 ||
			(incoming.get(routeOut.id)?.size ?? 0) !== 0 ||
			(outgoing.get(routeIn.id)?.size ?? 0) !== 0 ||
			(outgoing.get(routeOut.id)?.size ?? 0) === 0 ||
			(isExecution && outgoing.get(routeOut.id)?.size !== 1) ||
			isExecution !== (routeOut.data_type === "Execution")
		)
			continue;
		candidates.set(node.id, { node, input: routeIn, output: routeOut });
	}
	const chains: AutoRerouteChain[] = [];
	const visited = new Set<string>();
	// A malformed branch protects its entire generated tree, including valid
	// suffixes that would otherwise appear to start at a retained reroute.
	for (const id of [...generated.keys()].sort()) {
		if (visited.has(id)) continue;
		const component = [id];
		visited.add(id);
		for (let index = 0; index < component.length; index++) {
			for (const neighbor of neighbors.get(component[index]) ?? []) {
				if (visited.has(neighbor)) continue;
				visited.add(neighbor);
				component.push(neighbor);
			}
		}
		if (component.some((member) => !candidates.has(member))) continue;
		const members = new Set(component);
		const roots = component.filter((member) => {
			const route = required(candidates.get(member));
			const source = [...required(incoming.get(route.input.id))][0];
			return !members.has(owners.get(source) ?? "");
		});
		if (roots.length !== 1) continue;
		const start = required(candidates.get(roots[0]));
		const sourcePin = [...required(incoming.get(start.input.id))][0];
		const sourceId = owners.get(sourcePin);
		const source = pins.get(sourcePin);
		if (
			!sourceId ||
			source?.pin_type !== "Output" ||
			(input.only && !input.only.has(sourceId))
		)
			continue;
		// The two boundary boxes of an open layer share a runtime pin owner.
		// A direct same-owner wire is invalid, so keep their reroute topology.
		const boundaryIds = input.currentLayer
			? new Set([`${input.currentLayer}-input`, `${input.currentLayer}-return`])
			: undefined;
		const paths: AutoRerouteChain[] = [];
		const parents = new Map<string, string | null>([[start.node.id, null]]);
		const terminals = new Set<string>();
		const queue = [start.node.id];
		let valid = true;
		for (let index = 0; index < queue.length && valid; index++) {
			const current = required(candidates.get(queue[index]));
			if (
				(current.input.data_type === "Execution") !==
				(source.data_type === "Execution")
			) {
				valid = false;
				break;
			}
			for (const targetPin of [
				...required(outgoing.get(current.output.id)),
			].sort()) {
				const targetId = owners.get(targetPin);
				const target = pins.get(targetPin);
				if (
					!targetId ||
					target?.pin_type !== "Input" ||
					(target.data_type === "Execution") !==
						(source.data_type === "Execution")
				) {
					valid = false;
					break;
				}
				if (members.has(targetId)) {
					if (
						candidates.get(targetId)?.input.id !== targetPin ||
						parents.has(targetId)
					) {
						valid = false;
						break;
					}
					parents.set(targetId, current.node.id);
					queue.push(targetId);
					continue;
				}
				if (
					sourceId === targetId ||
					(boundaryIds?.has(sourceId) && boundaryIds.has(targetId)) ||
					(input.only && !input.only.has(targetId)) ||
					terminals.has(targetPin) ||
					(target.data_type !== "Execution" &&
						incoming.get(targetPin)?.size !== 1)
				) {
					valid = false;
					break;
				}
				terminals.add(targetPin);
				const nodes: INode[] = [];
				let member: string | null = current.node.id;
				while (member) {
					nodes.push(required(generated.get(member)));
					member = parents.get(member) ?? null;
				}
				paths.push({
					from: sourceId,
					to: targetId,
					fromPin: sourcePin,
					toPin: targetPin,
					nodes: nodes.reverse(),
				});
			}
		}
		if (valid && parents.size === component.length) chains.push(...paths);
	}
	if (!chains.length) return { input, chains };
	const removed = new Set(
		chains.flatMap((chain) => chain.nodes.map((node) => node.id)),
	);
	const updates = new Map<string, IPin>();
	for (const chain of chains) {
		const first = required(
			Object.values(chain.nodes[0].pins).find((pin) => pin.name === "route_in"),
		);
		const last = required(
			Object.values(chain.nodes[chain.nodes.length - 1].pins).find(
				(pin) => pin.name === "route_out",
			),
		);
		const source = structuredClone(
			updates.get(chain.fromPin) ?? required(pins.get(chain.fromPin)),
		);
		const target = structuredClone(
			updates.get(chain.toPin) ?? required(pins.get(chain.toPin)),
		);
		source.connected_to = [
			...new Set([
				...source.connected_to.filter((id) => id !== first.id),
				chain.toPin,
			]),
		];
		target.depends_on = [
			...new Set([
				...target.depends_on.filter((id) => id !== last.id),
				chain.fromPin,
			]),
		];
		updates.set(source.id, source);
		updates.set(target.id, target);
	}
	const updatePins = (values: Record<string, IPin>) =>
		Object.fromEntries(
			Object.entries(values).map(([id, pin]) => [id, updates.get(id) ?? pin]),
		);
	const boardLayers = input.boardLayers ? { ...input.boardLayers } : undefined;
	if (boardLayers) {
		for (const entity of input.layerEntities) {
			const layer = boardLayers[entity.id];
			if (layer)
				boardLayers[entity.id] = { ...layer, pins: updatePins(layer.pins) };
		}
	}
	return {
		chains,
		input: {
			...input,
			layerNodes: input.layerNodes
				.filter((node) => !removed.has(node.id))
				.map((node) => ({ ...node, pins: updatePins(node.pins) })),
			boardLayers,
			only: input.only
				? new Set([...input.only].filter((id) => !removed.has(id)))
				: undefined,
			obstacles: input.obstacles?.filter(
				(box) => !box.id || !removed.has(box.id),
			),
		},
	};
}

function required<T>(value: T | null | undefined): T {
	if (value === undefined || value === null)
		throw new Error("Missing reroute graph value");
	return value;
}
