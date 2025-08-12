import { createId } from "@paralleldrive/cuid2";
import { CopyIcon } from "lucide-react";
import { toast } from "sonner";
import { typeToColor } from "../components/flow/utils";
import {
	copyPasteCommand,
	removeLayerCommand,
	upsertCommentCommand,
	upsertLayerCommand,
} from "./command/generic-command";
import { toastSuccess } from "./messages";
import type { IGenericCommand, IValueType } from "./schema";
import {
	type IBoard,
	type IComment,
	ICommentType,
	type ILayer,
} from "./schema/flow/board";
import { IVariableType } from "./schema/flow/node";
import type { INode } from "./schema/flow/node";
import type { IPin, IPinType } from "./schema/flow/pin";

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
	layer?: string;
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
		layer: node.layer ?? undefined,
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
		layer: node.layer ?? "",
	};
}

export function isValidConnection(
	connection: any,
	cache: Map<string, [IPin, INode, boolean]>,
	refs: { [key: string]: string },
) {
	const [sourcePin, sourceNode] = cache.get(connection.sourceHandle) || [];
	const [targetPin, targetNode] = cache.get(connection.targetHandle) || [];

	if (!sourcePin || !targetPin) {
		console.warn(
			`Invalid connection: source or target pin not found for ${connection.sourceHandle} or ${connection.targetHandle}`,
		);
		return false;
	}
	if (!sourceNode || !targetNode) {
		console.warn(
			`Invalid connection: source or target node not found for ${connection.sourceHandle} or ${connection.targetHandle}`,
		);
		return false;
	}

	if (sourceNode.id === targetNode.id) {
		console.warn(
			`Invalid connection: source and target nodes are the same (${sourceNode.id})`,
		);
		return false;
	}

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
		targetPin.options?.enforce_generic_value_type ||
		sourcePin.options?.enforce_generic_value_type
	) {
		if (targetPin.value_type !== sourcePin.value_type) return false;
	}

	if (
		(sourcePin.data_type === "Generic" || targetPin.data_type === "Generic") &&
		sourcePin.data_type !== "Execution" &&
		targetPin.data_type !== "Execution"
	)
		return true;

	if (
		(targetPin.options?.enforce_schema || sourcePin.options?.enforce_schema) &&
		sourcePin.name !== "value_ref" &&
		targetPin.name !== "value_ref" &&
		sourcePin.name !== "value_in" &&
		targetPin.name !== "value_in" &&
		sourcePin.data_type !== "Generic" &&
		targetPin.data_type !== "Generic"
	) {
		if (!sourcePin.schema || !targetPin.schema) return false;
		if (schemaSource !== schemaTarget) return false;
	}

	if (sourcePin.value_type !== targetPin.value_type) return false;
	if (sourcePin.data_type !== targetPin.data_type) return false;

	return true;
}

export function parseBoard(
	board: IBoard,
	appId: string,
	handleCopy: (event?: ClipboardEvent) => void,
	pushLayer: (layer: ILayer) => void,
	executeBoard: (node: INode, payload?: object) => Promise<void>,
	executeCommand: (command: IGenericCommand, append: boolean) => Promise<any>,
	selected: Set<string>,
	connectionMode?: string,
	oldNodes?: any[],
	oldEdges?: any[],
	currentLayer?: string,
) {
	const nodes: any[] = [];
	const edges: any[] = [];
	const cache = new Map<string, [IPin, INode, boolean]>();
	const oldNodesMap = new Map<number, any>();
	const oldEdgesMap = new Map<string, any>();

	for (const oldNode of oldNodes ?? []) {
		if (oldNode.data?.hash) oldNodesMap.set(oldNode.data?.hash, oldNode);
	}

	for (const edge of oldEdges ?? []) {
		oldEdgesMap.set(edge.id, edge);
	}

	for (const node of Object.values(board.nodes)) {
		const nodeLayer = (node.layer ?? "") === "" ? undefined : node.layer;
		for (const pin of Object.values(node.pins)) {
			cache.set(pin.id, [pin, node, nodeLayer === currentLayer]);
		}
		if (nodeLayer !== currentLayer) continue;
		const hash = node.hash ?? -1;
		const oldNode = hash === -1 ? undefined : oldNodesMap.get(hash);
		if (oldNode && !oldNode?.data?.ghost) {
			nodes.push(oldNode);
		} else {
			nodes.push({
				id: node.id,
				type: "node",
				zIndex: 20,
				position: {
					x: node.coordinates?.[0] ?? 0,
					y: node.coordinates?.[1] ?? 0,
				},
				data: {
					label: node.name,
					node: node,
					hash: hash,
					ghost: false,
					boardId: board.id,
					appId: appId,
					onExecute: async (node: INode, payload?: object) => {
						await executeBoard(node, payload);
					},
					onCopy: async () => {
						handleCopy();
					},
				},
				selected: selected.has(node.id),
			});
		}
	}

	const activeLayer = new Set();
	if (board.layers)
		for (const layer of Object.values(board.layers)) {
			const parentLayer =
				(layer.parent_id ?? "") === "" ? undefined : layer.parent_id;
			if (parentLayer !== currentLayer) continue;

			const lookup: Record<string, INode> = {};
			if (layer.pins)
				for (const pin of Object.values(layer.pins)) {
					const [_, node] = cache.get(pin.id) || [];
					if (node) lookup[pin.id] = node;
				}

			activeLayer.add(layer.id);
			nodes.push({
				id: layer.id,
				type: "layerNode",
				position: { x: layer.coordinates[0], y: layer.coordinates[1] },
				zIndex: 19,
				data: {
					label: layer.id,
					boardId: board.id,
					appId: appId,
					layer: layer,
					pinLookup: lookup,
					pushLayer: async (layer: ILayer) => {
						pushLayer(layer);
					},
					onLayerUpdate: async (layer: ILayer) => {
						const command = upsertLayerCommand({
							current_layer: currentLayer,
							layer: layer,
							node_ids: [],
						});
						await executeCommand(command, false);
					},
					onLayerRemove: async (layer: ILayer, preserve_nodes: boolean) => {
						const command = removeLayerCommand({
							layer,
							child_layers: [],
							layer_nodes: [],
							layers: [],
							nodes: [],
							preserve_nodes,
						});
						await executeCommand(command, false);
					},
				},
				selected: selected.has(layer.id),
			});
		}

	const currentLayerRef: ILayer | undefined = board.layers[currentLayer ?? ""];
	for (const [pin, node, visible] of cache.values()) {
		if (pin.connected_to.length === 0) continue;

		for (const connectedTo of pin.connected_to) {
			const shadowNodes = new Map();
			const [conntectedPin, connectedNode, connectedVisible] =
				cache.get(connectedTo) || [];
			const connectedLayer = board.layers[connectedNode?.layer ?? ""];
			if (!visible && !connectedVisible) continue;
			if (!conntectedPin || !connectedNode) continue;

			if (
				visible !== connectedVisible &&
				(connectedLayer?.parent_id ?? "") !== (currentLayer ?? "")
			) {
				if (!visible && node.layer === currentLayerRef.parent_id) {
					let coordinates = node.coordinates ?? [0, 0, 0];

					if (currentLayerRef?.nodes[node.id]) {
						coordinates = currentLayerRef.nodes[node.id]?.coordinates ?? [
							0, 0, 0,
						];
					}

					nodes.push({
						id: node.id,
						type: "node",
						deletable: false,
						style: { opacity: 0.5 },
						zIndex: 20,
						position: {
							x: coordinates?.[0] ?? 0,
							y: coordinates?.[1] ?? 0,
						},
						data: {
							label: node.name,
							node: node,
							hash: node.hash ?? -1,
							boardId: board.id,
							appId: appId,
							ghost: true,
							onExecute: async (node: INode, payload?: object) => {
								await executeBoard(node, payload);
							},
							onCopy: async () => {
								handleCopy();
							},
						},
						selected: selected.has(node.id),
					});
					shadowNodes.set(node.id, node);
				} else if (
					!connectedVisible &&
					connectedNode.layer === currentLayerRef.parent_id
				) {
					let coordinates = connectedNode.coordinates ?? [0, 0, 0];

					if (currentLayerRef?.nodes[connectedNode.id]) {
						coordinates = currentLayerRef.nodes[connectedNode.id]
							?.coordinates ?? [0, 0, 0];
					}

					nodes.push({
						id: connectedNode.id,
						type: "node",
						zIndex: 20,
						deletable: false,
						style: { opacity: 0.5 },
						position: {
							x: coordinates?.[0] ?? 0,
							y: coordinates?.[1] ?? 0,
						},
						data: {
							label: connectedNode.name,
							node: connectedNode,
							boardId: board.id,
							appId: appId,
							hash: connectedNode.hash ?? -1,
							ghost: true,
							onExecute: async (node: INode, payload?: object) => {
								await executeBoard(node, payload);
							},
							onCopy: async () => {
								handleCopy();
							},
						},
						selected: selected.has(connectedNode.id),
					});
					shadowNodes.set(connectedNode.id, connectedNode);
				}
			}

			const edge = oldEdgesMap.get(`${pin.id}-${connectedTo}`);

			if (
				edge &&
				visible === connectedVisible &&
				edge.data.fromLayer === node.layer &&
				edge.data.toLayer === connectedNode.layer
			) {
				edges.push(edge);
				continue;
			}

			const sourceNode = shadowNodes.has(node.id)
				? node.id
				: activeLayer.has(node.layer ?? "")
					? node.layer
					: node.id;

			const connectedNodeId = shadowNodes.has(connectedNode.id)
				? connectedNode.id
				: activeLayer.has(connectedNode.layer ?? "")
					? connectedNode.layer
					: connectedNode.id;

			if (pin.id && conntectedPin.id)
				edges.push({
					id: `${pin.id}-${conntectedPin.id}`,
					source: sourceNode,
					sourceHandle: pin.id,
					zIndex: 18,
					data: {
						fromLayer: node.layer,
						toLayer: connectedNode.layer,
					},
					animated: pin.data_type !== "Execution",
					reconnectable: true,
					target: connectedNodeId,
					targetHandle: conntectedPin.id,
					style: { stroke: typeToColor(pin.data_type) },
					type: connectionMode ?? "default",
					data_type: pin.data_type,
					selected: selected.has(`${pin.id}-${connectedTo}`),
				});
		}
	}

	for (const comment of Object.values(board.comments)) {
		const commentLayer =
			(comment.layer ?? "") === "" ? undefined : comment.layer;
		if (commentLayer !== currentLayer) continue;
		const hash = comment.hash ?? -1;
		const oldNode = hash === -1 ? undefined : oldNodesMap.get(hash);
		if (oldNode) {
			nodes.push(oldNode);
			continue;
		}

		nodes.push({
			id: comment.id,
			type: "commentNode",
			position: { x: comment.coordinates[0], y: comment.coordinates[1] },
			width: comment.width ?? 200,
			height: comment.height ?? 80,
			zIndex: comment.z_index ?? 1,
			draggable: !(comment.is_locked ?? false),
			data: {
				label: comment.id,
				boardId: board.id,
				hash: hash,
				comment: {...comment, is_locked: comment.is_locked ?? false},
				onUpsert: (comment: IComment) => {
					const command = upsertCommentCommand({
						comment: comment,
						current_layer: currentLayer,
					});
					executeCommand(command, false);
				},
			},
			selected: selected.has(comment.id),
		});
	}
	return { nodes, edges, cache };
}

export function handleCopy(
	nodes: any[],
	board: IBoard,
	cursorPosition?: { x: number; y: number },
	event?: ClipboardEvent,
	currentLayer?: string,
) {
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

	const allLayer = Object.values(board.layers);

	const startLayer: ILayer[] = nodes
		.filter((node) => node.selected && node.type === "layerNode")
		.map((node) => node.data.layer);

	const foundLayer = new Map<string, ILayer>(
		startLayer.map((layer) => [layer.id, { ...layer, parent_id: undefined }]),
	);

	let previousSize = 0;

	while (previousSize < foundLayer.size) {
		previousSize = foundLayer.size;
		for (const layer of allLayer) {
			if (foundLayer.has(layer.id)) continue;
			if (!layer.parent_id || layer.parent_id === "") continue;
			if (foundLayer.has(layer.parent_id)) {
				foundLayer.set(layer.id, layer);
			}
		}
	}

	const selected = new Set(
		nodes.filter((node) => node.selected).map((node) => node.id),
	);
	const selectedNodes = Object.values(board.nodes)
		.filter((node) => selected.has(node.id) || foundLayer.has(node.layer ?? ""))
		.map((node) =>
			serializeNode({
				...node,
				layer:
					(node.layer ?? "") === (currentLayer ?? "") ? undefined : node.layer,
			}),
		);

	const selectedComments = Object.values(board.comments)
		.filter(
			(comment) =>
				selected.has(comment.id) || foundLayer.has(comment.layer ?? ""),
		)
		.map((comment) => ({
			...comment,
			layer:
				(comment.layer ?? "") === (currentLayer ?? "")
					? undefined
					: comment.layer,
		}));

	try {
		navigator.clipboard.writeText(
			JSON.stringify(
				{
					nodes: selectedNodes,
					comments: selectedComments,
					cursorPosition,
					layers: Array.from(foundLayer.values()),
				},
				null,
				2,
			),
		);
		toastSuccess("Nodes copied to clipboard", <CopyIcon className="w-4 h-4" />);
		return;
	} catch (error) {
		toast.error("Failed to copy nodes to clipboard");
		throw error;
	}
}

export async function handlePaste(
	event: ClipboardEvent,
	cursorPosition: { x: number; y: number },
	boardId: string,
	executeCommand: (command: IGenericCommand, append?: boolean) => Promise<any>,
	currentLayer?: string,
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
		const oldPosition = data.cursorPosition;
		const nodes: any[] = data.nodes.map((node: ISerializedNode) =>
			deserializeNode(node),
		);
		const comments: any[] = data.comments;
		const layers: ILayer[] = data.layers ?? [];

		const command = copyPasteCommand({
			original_comments: comments,
			original_nodes: nodes,
			original_layers: layers,
			new_comments: [],
			new_nodes: [],
			new_layers: [],
			current_layer: currentLayer,
			old_mouse: oldPosition ? [oldPosition.x, oldPosition.y, 0] : undefined,
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
			current_layer: currentLayer,
		});

		await executeCommand(command);
		return;
	} catch (error) {}
}
