import type { Connection, Edge } from "@xyflow/react";
import {
	connectPinsCommand,
	disconnectPinsCommand,
	updateNodeCommand,
} from "../command/generic-command";
import { isValidConnection } from "../flow-board-utils";
import type { ILayer } from "../schema/flow/board";
import type { IGenericCommand } from "../schema/flow/board/commands/generic-command";
import type { INode } from "../schema/flow/node";
import { type IPin, IPinType, IVariableType } from "../schema/flow/pin";

type PinCache = Map<string, [IPin, INode | ILayer, boolean]>;
interface Handles {
	sourceHandle?: string | null;
	targetHandle?: string | null;
}
interface PinEndpoint {
	pin: IPin;
	owner: INode | ILayer;
}
interface PinConnection {
	kind: "pins";
	output: PinEndpoint;
	input: PinEndpoint;
}
interface ReferenceConnection {
	kind: "reference";
	output: INode;
	input: INode;
}
type ResolvedConnection = PinConnection | ReferenceConnection;

interface ConnectionCommandInput {
	connection: Connection;
	oldEdge?: Edge;
	boardNodes: Record<string, INode>;
	pinCache: PinCache;
	refs: Record<string, string>;
}

function resolveConnection(
	{ sourceHandle, targetHandle }: Handles,
	boardNodes: Record<string, INode>,
	pinCache: PinCache,
): ResolvedConnection | undefined {
	if (!sourceHandle || !targetHandle || sourceHandle === targetHandle) return;
	const handles = [sourceHandle, targetHandle];
	if (handles.some((handle) => /^ref_(in|out)_/.test(handle))) {
		const outputId = handles
			.find((handle) => handle.startsWith("ref_out_"))
			?.slice(8);
		const inputId = handles
			.find((handle) => handle.startsWith("ref_in_"))
			?.slice(7);
		if (!outputId || !inputId || outputId === inputId) return;
		const output = boardNodes[outputId];
		const input = boardNodes[inputId];
		if (!output || !input) return;
		return { kind: "reference", output, input };
	}
	const source = pinCache.get(sourceHandle);
	const target = pinCache.get(targetHandle);
	if (!source || !target || source[1].id === target[1].id) return;
	if (source[0].pin_type === target[0].pin_type) return;
	// The cache already inverts inner-layer pins and retains their canonical owner.
	const [output, input] =
		source[0].pin_type === IPinType.Output
			? [source, target]
			: [target, source];
	return {
		kind: "pins",
		output: { pin: output[0], owner: output[1] },
		input: { pin: input[0], owner: input[1] },
	};
}

function key(connection: ResolvedConnection): string {
	return connection.kind === "reference"
		? JSON.stringify(["reference", connection.output.id, connection.input.id])
		: JSON.stringify([
				"pins",
				connection.output.pin.id,
				connection.input.pin.id,
			]);
}

function exists(connection: ResolvedConnection): boolean {
	return connection.kind === "reference"
		? (connection.output.fn_refs?.fn_refs.includes(connection.input.id) ??
				false)
		: connection.output.pin.connected_to.includes(connection.input.pin.id) ||
				connection.input.pin.depends_on.includes(connection.output.pin.id);
}

function pinCommand(connection: PinConnection) {
	return {
		from_node: connection.output.owner.id,
		from_pin: connection.output.pin.id,
		to_node: connection.input.owner.id,
		to_pin: connection.input.pin.id,
	};
}

function referenceCommands(
	next?: ReferenceConnection,
	previous?: ReferenceConnection,
): IGenericCommand[] {
	const changed = new Map<string, { node: INode; refs: string[] }>();
	if (previous) {
		changed.set(previous.output.id, {
			node: previous.output,
			refs: (previous.output.fn_refs?.fn_refs ?? []).filter(
				(id) => id !== previous.input.id,
			),
		});
	}
	if (next) {
		const target = changed.get(next.output.id) ?? {
			node: next.output,
			refs: [...(next.output.fn_refs?.fn_refs ?? [])],
		};
		if (!target.refs.includes(next.input.id)) target.refs.push(next.input.id);
		changed.set(next.output.id, target);
	}
	return [...changed.values()]
		.filter(({ node, refs }) => {
			const original = node.fn_refs?.fn_refs ?? [];
			return (
				original.length !== refs.length ||
				original.some((id, index) => id !== refs[index])
			);
		})
		.map(({ node, refs }) =>
			updateNodeCommand({
				node: {
					...node,
					fn_refs: {
						...node.fn_refs,
						can_reference_fns: node.fn_refs?.can_reference_fns ?? false,
						can_be_referenced_by_fns:
							node.fn_refs?.can_be_referenced_by_fns ?? false,
						fn_refs: refs,
					},
				},
			}),
		);
}

/** Build one undoable gesture. Undefined rejects a stale/invalid proposal; [] is a no-op. */
export function buildConnectionCommands({
	connection,
	oldEdge,
	boardNodes,
	pinCache,
	refs,
}: ConnectionCommandInput): IGenericCommand[] | undefined {
	if (!isValidConnection(connection, pinCache, refs)) return;
	const next = resolveConnection(connection, boardNodes, pinCache);
	const previous = oldEdge
		? resolveConnection(oldEdge, boardNodes, pinCache)
		: undefined;
	if (!next || (oldEdge && (!previous || !exists(previous)))) return;
	if (previous && previous.kind !== next.kind) return;
	if (previous && key(previous) === key(next)) return [];
	if (!previous && exists(next)) return [];
	if (next.kind === "reference") {
		return referenceCommands(
			next,
			previous?.kind === "reference" ? previous : undefined,
		);
	}

	const removed = new Map<string, PinConnection>();
	if (previous?.kind === "pins") removed.set(key(previous), previous);
	if (exists(next))
		return [...removed.values()].map((link) =>
			disconnectPinsCommand(pinCommand(link)),
		);

	// ConnectPin silently displaces occupied endpoints, but its undo only removes the new wire.
	// Explicit disconnects record the displaced wires so reversing the batch restores them.
	const execution = next.output.pin.data_type === IVariableType.Execution;
	const occupied = execution
		? next.output.pin.connected_to
		: next.input.pin.depends_on;
	for (const pinId of occupied) {
		const displaced = pinCache.get(pinId);
		if (!displaced) return;
		const endpoint = { pin: displaced[0], owner: displaced[1] };
		const link: PinConnection = {
			kind: "pins",
			output: execution ? next.output : endpoint,
			input: execution ? endpoint : next.input,
		};
		removed.set(key(link), link);
	}
	return [
		...[...removed.values()].map((link) =>
			disconnectPinsCommand(pinCommand(link)),
		),
		connectPinsCommand(pinCommand(next)),
	];
}

export function buildDisconnectionCommands({
	edge,
	boardNodes,
	pinCache,
}: {
	edge: Edge;
	boardNodes: Record<string, INode>;
	pinCache: PinCache;
}): IGenericCommand[] | undefined {
	const connection = resolveConnection(edge, boardNodes, pinCache);
	if (!connection) return;
	if (!exists(connection)) return [];
	return connection.kind === "reference"
		? referenceCommands(undefined, connection)
		: [disconnectPinsCommand(pinCommand(connection))];
}
