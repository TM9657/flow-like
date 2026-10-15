import {
	addNodeCommand,
	connectPinsCommand,
	disconnectPinsCommand,
	moveNodeCommand,
	removeNodeCommand,
} from "../command/generic-command";
import type { IGenericCommand } from "../schema/flow/board/commands/generic-command";
import type { INode, IPin } from "../schema/flow/node";
import type { ILayer } from "../schema/flow/run";
import type { AutoRerouteChain } from "./normalize-reroutes";
import type { WireRoute } from "./route";
interface RerouteCommandInput {
	routes: readonly WireRoute[];
	chains: readonly AutoRerouteChain[];
	reroute?: INode;
	currentLayer: string | undefined;
	pinCache: ReadonlyMap<string, [IPin, INode | ILayer, boolean]>;
}
const routePin = (node: INode, name: string) =>
	Object.values(node.pins).find((pin) => pin.name === name);
const key = (route: {
	fromPin: string;
	toPin: string;
}) => `${route.fromPin}->${route.toPin}`;
interface Connection {
	from_node: string;
	from_pin: string;
	to_node: string;
	to_pin: string;
}

interface DesiredNode {
	key: string;
	point: { x: number; y: number };
	fromPin: IPin;
	candidates: Map<string, { node: INode; uses: number }>;
	node?: INode;
}

/** Build one undoable edit, reusing generated nodes whenever the route allows it. */
export function buildAutoRerouteCommands({
	routes,
	chains,
	reroute,
	currentLayer,
	pinCache,
}: RerouteCommandInput): IGenericCommand[] {
	const oldChains = new Map(chains.map((chain) => [key(chain), chain]));
	const planned = new Map(routes.map((route) => [key(route), route]));
	for (const chain of chains) {
		if (!planned.has(key(chain)))
			planned.set(key(chain), { ...chain, waypoints: [], unresolved: true });
	}

	// An unresolved leaf keeps its whole shared tree, including the geometry used by siblings.
	const nodeRoutes = new Map<string, Set<string>>();
	for (const chain of chains) {
		for (const node of chain.nodes) {
			const owners = nodeRoutes.get(node.id) ?? new Set<string>();
			owners.add(key(chain));
			nodeRoutes.set(node.id, owners);
		}
	}
	const protectedRoutes = new Set<string>();
	const queue: string[] = [];
	for (const [routeKey, route] of planned) {
		if (
			route.unresolved ||
			!pinCache.has(route.fromPin) ||
			!pinCache.has(route.toPin)
		) {
			protectedRoutes.add(routeKey);
			queue.push(routeKey);
		}
	}
	for (let index = 0; index < queue.length; index++) {
		for (const node of oldChains.get(queue[index])?.nodes ?? []) {
			for (const sibling of nodeRoutes.get(node.id) ?? []) {
				if (protectedRoutes.has(sibling)) continue;
				protectedRoutes.add(sibling);
				queue.push(sibling);
			}
		}
	}

	const active = [...planned.entries()]
		.filter(([routeKey]) => !protectedRoutes.has(routeKey))
		.sort(([a], [b]) => a.localeCompare(b));
	const desired = new Map<string, DesiredNode>();
	const paths = new Map<string, DesiredNode[]>();
	const oldNodes = new Map<string, INode>();
	for (const [routeKey, route] of active) {
		const [fromPin] = required(pinCache.get(route.fromPin));
		const previous = oldChains.get(routeKey)?.nodes ?? [];
		for (const node of previous) oldNodes.set(node.id, node);
		const path: DesiredNode[] = [];
		for (const [index, point] of route.waypoints.entries()) {
			const shared = point.shared && fromPin.data_type !== "Execution";
			const nodeKey = shared
				? JSON.stringify(["shared", route.fromPin, point.shared])
				: JSON.stringify(["private", routeKey, index]);
			let next = desired.get(nodeKey);
			if (!next) {
				next = { key: nodeKey, point, fromPin, candidates: new Map() };
				desired.set(nodeKey, next);
			}
			const existing = previous[index];
			if (existing) {
				const candidate = next.candidates.get(existing.id);
				next.candidates.set(existing.id, {
					node: existing,
					uses: (candidate?.uses ?? 0) + 1,
				});
			}
			path.push(next);
		}
		paths.set(routeKey, path);
	}

	const commands: IGenericCommand[] = [];
	const used = new Set<string>();
	const ordered = [...desired.values()].sort((a, b) => {
		const uses = (node: DesiredNode) =>
			[...node.candidates.values()].reduce(
				(sum, candidate) => sum + candidate.uses,
				0,
			);
		return uses(b) - uses(a) || a.key.localeCompare(b.key);
	});
	for (const next of ordered) {
		const candidates = [...next.candidates.values()].sort(
			(a, b) => b.uses - a.uses || a.node.id.localeCompare(b.node.id),
		);
		const existing = candidates.find(({ node }) => !used.has(node.id))?.node;
		const coordinates = [next.point.x, next.point.y, 0];
		if (existing) {
			next.node = existing;
			used.add(existing.id);
			if (
				existing.coordinates?.[0] !== coordinates[0] ||
				existing.coordinates?.[1] !== coordinates[1]
			) {
				commands.push(
					moveNodeCommand({
						node_id: existing.id,
						from_coordinates: existing.coordinates ?? [0, 0, 0],
						to_coordinates: coordinates,
						current_layer: currentLayer,
					}),
				);
			}
			continue;
		}
		if (
			!reroute ||
			!routePin(reroute, "route_in") ||
			!routePin(reroute, "route_out")
		)
			throw new Error("The reroute node is missing its routing pins.");
		const template = structuredClone(reroute);
		template.auto_reroute = true;
		template.coordinates = coordinates;
		for (const pin of Object.values(template.pins)) {
			pin.data_type = next.fromPin.data_type;
			pin.value_type = next.fromPin.value_type;
			pin.schema = next.fromPin.schema;
			pin.options = next.fromPin.options
				? structuredClone(next.fromPin.options)
				: undefined;
			pin.connected_to = [];
			pin.depends_on = [];
			pin.default_value = null;
		}
		const added = addNodeCommand({
			node: template,
			current_layer: currentLayer,
		});
		commands.push(added.command);
		next.node = added.node;
	}

	const connections = (route: WireRoute, nodes: INode[]) => {
		const [fromPin, fromNode] = required(pinCache.get(route.fromPin));
		const [toPin, toNode] = required(pinCache.get(route.toPin));
		const wires: Connection[] = [];
		let source = { node: fromNode.id, pin: fromPin.id };
		for (const node of nodes) {
			wires.push({
				from_node: source.node,
				from_pin: source.pin,
				to_node: node.id,
				to_pin: required(routePin(node, "route_in")).id,
			});
			source = {
				node: node.id,
				pin: required(routePin(node, "route_out")).id,
			};
		}
		wires.push({
			from_node: source.node,
			from_pin: source.pin,
			to_node: toNode.id,
			to_pin: toPin.id,
		});
		return wires;
	};
	const oldWires = new Map<string, Connection>();
	const nextWires = new Map<string, Connection>();
	const connectionKey = (wire: Connection) =>
		`${wire.from_pin}->${wire.to_pin}`;
	for (const [routeKey, route] of active) {
		for (const wire of connections(route, oldChains.get(routeKey)?.nodes ?? []))
			oldWires.set(connectionKey(wire), wire);
		for (const wire of connections(
			route,
			required(paths.get(routeKey)).map((entry) => required(entry.node)),
		))
			nextWires.set(connectionKey(wire), wire);
	}
	for (const [wireKey, wire] of oldWires) {
		if (!nextWires.has(wireKey)) commands.push(disconnectPinsCommand(wire));
	}
	for (const node of oldNodes.values()) {
		if (used.has(node.id)) continue;
		const disconnected = structuredClone(node);
		for (const pin of Object.values(disconnected.pins)) {
			pin.connected_to = [];
			pin.depends_on = [];
		}
		commands.push(
			removeNodeCommand({ node: disconnected, connected_nodes: [] }),
		);
	}
	for (const [wireKey, wire] of nextWires) {
		if (!oldWires.has(wireKey)) commands.push(connectPinsCommand(wire));
	}
	return commands;
}

function required<T>(value: T | null | undefined): T {
	if (value === undefined || value === null)
		throw new Error("Missing reroute graph value");
	return value;
}
