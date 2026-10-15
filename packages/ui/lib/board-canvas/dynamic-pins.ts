import {
	disconnectPinsCommand,
	updateNodeCommand,
} from "../command/generic-command";
import type { IBoard } from "../schema/flow/board";
import type { IGenericCommand } from "../schema/flow/board/commands/generic-command";
import type { INode } from "../schema/flow/node";
import type { IPin } from "../schema/flow/pin";

interface PinOwner {
	ownerId: string;
	pin: IPin;
}

/** Disconnect before deletion so reversing the returned batch restores the pin before its wires. */
export function buildDynamicPinRemovalCommands({
	node,
	pinId,
	board,
	position,
}: {
	node: INode;
	pinId: string;
	board: Pick<IBoard, "nodes" | "layers">;
	position: { x: number; y: number };
}): IGenericCommand[] | undefined {
	const current = board.nodes[node.id];
	const requested = node.pins[pinId];
	const pin = current?.pins[pinId];
	if (
		!pin ||
		!requested ||
		pin.name !== requested.name ||
		pin.pin_type !== requested.pin_type ||
		!Number.isFinite(position.x) ||
		!Number.isFinite(position.y)
	)
		return;
	const family = Object.values(current.pins)
		.filter((peer) => peer.name === pin.name && peer.pin_type === pin.pin_type)
		.sort((a, b) => a.index - b.index);
	// The original pair defines a dynamic family and cannot be removed.
	if (family.length <= 2 || family.findIndex((peer) => peer.id === pinId) < 2)
		return;

	const owners = new Map<string, PinOwner>();
	const ambiguous = new Set<string>();
	for (const owner of [
		...Object.values(board.nodes),
		...Object.values(board.layers),
	]) {
		for (const peer of Object.values(owner.pins)) {
			if (owners.has(peer.id)) ambiguous.add(peer.id);
			owners.set(peer.id, { ownerId: owner.id, pin: peer });
		}
	}
	if (ambiguous.has(pinId)) return;
	const removed: PinOwner = { ownerId: current.id, pin };
	const links = new Map<string, [PinOwner, PinOwner]>();
	const addLink = (from: PinOwner | undefined, to: PinOwner | undefined) => {
		if (!from || !to || from.ownerId === to.ownerId) return;
		links.set(JSON.stringify([from.pin.id, to.pin.id]), [from, to]);
	};
	for (const id of pin.connected_to) {
		if (ambiguous.has(id)) return;
		addLink(removed, owners.get(id));
	}
	for (const id of pin.depends_on) {
		if (ambiguous.has(id)) return;
		addLink(owners.get(id), removed);
	}
	// Also find peers with a one-sided reference to the pin being removed.
	for (const owner of owners.values()) {
		if (owner.pin.connected_to.includes(pinId)) addLink(owner, removed);
		if (owner.pin.depends_on.includes(pinId)) addLink(removed, owner);
	}
	for (const [from, to] of links.values()) {
		if (ambiguous.has(from.pin.id) || ambiguous.has(to.pin.id)) return;
	}

	const pins: INode["pins"] = {};
	for (const direction of ["Input", "Output"]) {
		const remaining = Object.values(current.pins)
			.filter((peer) => peer.id !== pinId && peer.pin_type === direction)
			.sort((a, b) => a.index - b.index);
		for (const [index, peer] of remaining.entries()) {
			pins[peer.id] = {
				...peer,
				index: index + 1,
				// Same-node links cannot use DisconnectPin: its undo rejects self-connections.
				connected_to: peer.connected_to.filter((id) => id !== pinId),
				depends_on: peer.depends_on.filter((id) => id !== pinId),
			};
		}
	}
	return [
		...[...links.values()].map(([from, to]) =>
			disconnectPinsCommand({
				from_node: from.ownerId,
				from_pin: from.pin.id,
				to_node: to.ownerId,
				to_pin: to.pin.id,
			}),
		),
		updateNodeCommand({
			node: {
				...current,
				pins,
				coordinates: [position.x, position.y, current.coordinates?.[2] ?? 0],
			},
		}),
	];
}
