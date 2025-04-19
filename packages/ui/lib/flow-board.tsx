import crypto from "crypto";
import { createId } from "@paralleldrive/cuid2";
import { CopyIcon } from "lucide-react";
import { toast } from "sonner";
import { typeToColor } from "../components/flow/utils";
import {
	copyPasteCommand,
	upsertCommentCommand,
} from "./command/generic-command";
import { toastSuccess } from "./messages";
import type { IGenericCommand, IValueType } from "./schema";
import { type IBoard, type IComment, ICommentType } from "./schema/flow/board";
import { IVariableType } from "./schema/flow/node";
import type { INode } from "./schema/flow/node";
import type { IPin, IPinType } from "./schema/flow/pin";
import type { IRun, ITrace } from "./schema/flow/run";

interface ISerializedPin {
	id: string;
	name: string;
	friendly_name: string;
	pin_type: IPinType;
	data_type: IVariableType;
	value_type: IValueType;
	depends_on: string[];
	connected_to: string[];
	default_value?: number[];
	index: number;
}
interface ISerializedNode {
	id: string;
	name: string;
	friendly_name: string;
	comment?: string;
	coordinates?: number[];
	pins: {
		[key: string]: ISerializedPin;
	};
}

function serializeNode(node: INode): ISerializedNode {
	const pins: {
		[key: string]: ISerializedPin;
	} = {};

	for (const pin of Object.values(node.pins)) {
		pins[pin.id] = {
			id: pin.id,
			name: pin.name,
			friendly_name: pin.friendly_name,
			pin_type: pin.pin_type,
			data_type: pin.data_type,
			value_type: pin.value_type,
			depends_on: pin.depends_on,
			connected_to: pin.connected_to,
			default_value: pin.default_value ?? undefined,
			index: pin.index,
		};
	}

	return {
		id: node.id,
		name: node.name,
		friendly_name: node.friendly_name,
		comment: node.comment ?? undefined,
		coordinates: node.coordinates ?? undefined,
		pins: pins,
	};
}

function deserializeNode(node: ISerializedNode): INode {
	const pins: {
		[key: string]: IPin;
	} = {};

	for (const pin of Object.values(node.pins)) {
		pins[pin.id] = {
			id: pin.id,
			name: pin.name,
			friendly_name: pin.friendly_name,
			pin_type: pin.pin_type,
			data_type: pin.data_type,
			value_type: pin.value_type,
			depends_on: pin.depends_on,
			connected_to: pin.connected_to,
			default_value: pin.default_value ?? undefined,
			index: pin.index,
			description: "",
			schema: "",
		};
	}

	return {
		id: node.id,
		category: "",
		name: node.name,
		description: "",
		friendly_name: node.friendly_name,
		coordinates: node.coordinates ?? [0, 0, 0],
		comment: node.comment ?? "",
		pins: pins,
	};
}

export function isValidConnection(
	connection: any,
	cache: Map<string, [IPin, INode]>,
	refs: { [key: string]: string },
) {
	const [sourcePin, sourceNode] = cache.get(connection.sourceHandle) || [];
	const [targetPin, targetNode] = cache.get(connection.targetHandle) || [];

	if (!sourcePin || !targetPin) return false;
	if (!sourceNode || !targetNode) return false;

	if (sourceNode.id === targetNode.id) return false;

	return doPinsMatch(sourcePin, targetPin, refs);
}

export function doPinsMatch(
	sourcePin: IPin,
	targetPin: IPin,
	refs: { [key: string]: string },
) {
	if (
		(sourcePin.name === "route_in" &&
			sourcePin.data_type === IVariableType.Generic) ||
		(targetPin.name === "route_in" &&
			targetPin.data_type === IVariableType.Generic)
	)
		return true;
	if (
		(targetPin.name === "route_out" &&
			targetPin.data_type === IVariableType.Generic) ||
		(sourcePin.name === "route_out" &&
			sourcePin.data_type === IVariableType.Generic)
	)
		return true;

	if (sourcePin.pin_type === targetPin.pin_type) return false;

	let schemaSource = sourcePin.schema;
	if (schemaSource) {
		schemaSource = refs[schemaSource] ?? schemaSource;
	}

	let schemaTarget = targetPin.schema;
	if (schemaTarget) {
		schemaTarget = refs[schemaTarget] ?? schemaTarget;
	}

	if (sourcePin.schema && targetPin.schema) {
		if (schemaSource !== schemaTarget) return false;
	}

	if (
		(targetPin.options?.enforce_schema || sourcePin.options?.enforce_schema) &&
		sourcePin.name !== "value_ref" &&
		targetPin.name !== "value_ref" &&
		sourcePin.name !== "value_in" &&
		targetPin.name !== "value_in"
	) {
		if (!sourcePin.schema || !targetPin.schema) return false;
		if (schemaSource !== schemaTarget) return false;
	}

	if (targetPin.options?.valid_values || sourcePin.options?.valid_values) {
		if (targetPin.value_type !== sourcePin.value_type) return false;
	}

	if (
		(sourcePin.data_type === "Generic" || targetPin.data_type === "Generic") &&
		sourcePin.data_type !== "Execution" &&
		targetPin.data_type !== "Execution"
	)
		return true;
	if (sourcePin.value_type !== targetPin.value_type) return false;
	if (sourcePin.data_type !== targetPin.data_type) return false;

	return true;
}

function hashNode(node: INode | IComment, traces?: ITrace[]) {
	const hash = crypto.createHash("md5");
	hash.update(JSON.stringify(node));
	if (traces) {
		hash.update(JSON.stringify(traces));
	}
	return hash.digest("hex");
}

export function parseBoard(
	board: IBoard,
	appId: string,
	executeBoard: (node: INode, payload?: object) => Promise<void>,
	openTraces: (node: INode, traces: ITrace[]) => Promise<void>,
	executeCommand: (command: IGenericCommand, append: boolean) => Promise<any>,
	selected: Set<string>,
	run?: IRun,
	connectionMode?: string,
	oldNodes?: any[],
	oldEdges?: any[],
) {
	const nodes: any[] = [];
	const edges: any[] = [];
	const cache = new Map<string, [IPin, INode]>();
	const traces = new Map<string, ITrace[]>();
	const oldNodesMap = new Map<string, any>();
	const oldEdgesMap = new Map<string, any>();

	for (const oldNode of oldNodes ?? []) {
		oldNodesMap.set(oldNode.data?.hash, oldNode);
	}

	for (const edge of oldEdges ?? []) {
		oldEdgesMap.set(edge.id, edge);
	}

	for (const trace of run?.traces ?? []) {
		if (traces.has(trace.node_id)) {
			traces.get(trace.node_id)?.push(trace);
			continue;
		}

		traces.set(trace.node_id, [trace]);
	}

	for (const node of Object.values(board.nodes)) {
		const hash = hashNode(node, traces.get(node.id));
		const oldNode = oldNodesMap.get(hash);
		if (oldNode) {
			nodes.push(oldNode);
		} else {
			nodes.push({
				id: node.id,
				type: "node",
				position: {
					x: node.coordinates?.[0] ?? 0,
					y: node.coordinates?.[1] ?? 0,
				},
				data: {
					label: node.name,
					node: node,
					hash: hash,
					boardId: board.id,
					appId: appId,
					onExecute: async (node: INode, payload?: object) => {
						await executeBoard(node, payload);
					},
					openTrace: async (traces: ITrace[]) => {
						await openTraces(node, traces);
					},
					traces: traces.get(node.id) || [],
				},
				selected: selected.has(node.id),
			});
		}

		for (const pin of Object.values(node.pins)) {
			cache.set(pin.id, [pin, node]);
		}
	}

	for (const [pin, node] of cache.values()) {
		if (pin.connected_to.length === 0) continue;

		for (const connectedTo of pin.connected_to) {
			const [conntectedPin, connectedNode] = cache.get(connectedTo) || [];
			if (!conntectedPin || !connectedNode) continue;

			const edge = oldEdgesMap.get(`${pin.id}-${connectedTo}`);

			if (edge) {
				edges.push(edge);
				continue;
			}

			edges.push({
				id: `${pin.id}-${connectedTo}`,
				source: node.id,
				sourceHandle: pin.id,
				animated: pin.data_type !== "Execution",
				reconnectable: true,
				target: connectedNode.id,
				targetHandle: conntectedPin.id,
				style: { stroke: typeToColor(pin.data_type) },
				type: connectionMode ?? "simplebezier",
				data_type: pin.data_type,
				selected: selected.has(`${pin.id}-${connectedTo}`),
			});
		}
	}

	for (const comment of Object.values(board.comments)) {
		const hash = hashNode(comment);
		const oldNode = oldNodesMap.get(hash);
		if (oldNode) {
			nodes.push(oldNode);
			continue;
		}

		nodes.push({
			id: comment.id,
			type: "commentNode",
			position: { x: comment.coordinates[0], y: comment.coordinates[1] },
			data: {
				label: comment.id,
				boardId: board.id,
				hash: hash,
				comment: comment,
				onUpsert: (comment: IComment) => {
					const command = upsertCommentCommand({
						comment: comment,
					});
					executeCommand(command, false);
				},
			},
			selected: selected.has(comment.id),
		});
	}

	return { nodes, edges, cache, traces };
}

export function handleCopy(nodes: any[], event?: ClipboardEvent) {
	const activeElement = document.activeElement;
	if (
		activeElement instanceof HTMLInputElement ||
		activeElement instanceof HTMLTextAreaElement ||
		(activeElement as any)?.isContentEditable
	) {
		return;
	}

	event?.preventDefault();
	event?.stopPropagation();

	const selectedNodes = nodes
		.filter((node: any) => node.selected && node.type === "node")
		.map((node: any) => serializeNode(node.data.node));

	const selectedComments: IComment[] = nodes
		.filter((node: any) => node.selected && node.type === "commentNode")
		.map((node: any) => node.data.comment);
	try {
		navigator.clipboard.writeText(
			JSON.stringify(
				{ nodes: selectedNodes, comments: selectedComments },
				null,
				2,
			),
		);
		toastSuccess("Nodes copied to clipboard", <CopyIcon className="w-4 h-4" />);
		return;
	} catch (error) {
		toast.error("Failed to copy nodes to clipboard");
	}
}

export async function handlePaste(
	event: ClipboardEvent,
	cursorPosition: { x: number; y: number },
	boardId: string,
	executeCommand: (command: IGenericCommand, append?: boolean) => Promise<any>,
) {
	const activeElement = document.activeElement;
	if (
		activeElement instanceof HTMLInputElement ||
		activeElement instanceof HTMLTextAreaElement ||
		(activeElement as any)?.isContentEditable
	) {
		return;
	}

	event.preventDefault();
	event.stopPropagation();
	try {
		const clipboard = await navigator.clipboard.readText();
		const data = JSON.parse(clipboard);
		if (!data) return;
		if (!data.nodes && !data.comments) return;
		const nodes: any[] = data.nodes.map((node: ISerializedNode) =>
			deserializeNode(node),
		);
		const comments: any[] = data.comments;

		const command = copyPasteCommand({
			original_comments: comments,
			original_nodes: nodes,
			new_comments: [],
			new_nodes: [],
			offset: [cursorPosition.x, cursorPosition.y, 0],
		});
		await executeCommand(command);
		return;
	} catch (error) {}

	try {
		const clipboard = await navigator.clipboard.readText();
		const comment: IComment = {
			comment_type: ICommentType.Text,
			content: clipboard,
			coordinates: [cursorPosition.x, cursorPosition.y, 0],
			id: createId(),
			timestamp: {
				nanos_since_epoch: 0,
				secs_since_epoch: 0,
			},
		};

		const command = upsertCommentCommand({
			comment: comment,
		});

		await executeCommand(command);
		return;
	} catch (error) {}
}
