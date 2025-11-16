import { createId } from "@paralleldrive/cuid2";
import { CopyIcon } from "lucide-react";
import type { RefObject } from "react";
import { toast } from "sonner";
import { InnerLayerNodeType } from "../components/flow/layer-inner-node";
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
import type { IFnRefs, INode } from "./schema/flow/node";
import { type IPin, IPinType } from "./schema/flow/pin";

export function hexToRgba(hex: string, alpha = 0.3): string {
	let c = hex.replace("#", "");
	if (c.length === 3) c = c[0] + c[0] + c[1] + c[1] + c[2] + c[2];
	const num = Number.parseInt(c, 16);
	const r = (num >> 16) & 255;
	const g = (num >> 8) & 255;
	const b = num & 255;
	return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function normalizeSelectionNodes(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	return value.filter(
		(nodeId: unknown): nodeId is string => typeof nodeId === "string",
	);
}

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
	fn_refs?: IFnRefs;
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
		fn_refs: node.fn_refs ?? undefined,
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
		fn_refs: node.fn_refs ?? undefined,
	};
}

export function isValidConnection(
	connection: any,
	cache: Map<string, [IPin, INode | ILayer, boolean]>,
	refs: { [key: string]: string },
) {
	const refIn =
		connection.sourceHandle.startsWith("ref_in_") ||
		connection.targetHandle.startsWith("ref_in_");
	const refOut =
		connection.sourceHandle.startsWith("ref_out_") ||
		connection.targetHandle.startsWith("ref_out_");

	if (refIn || refOut) {
		return refIn && refOut;
	}

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

	return doPinsMatch(sourcePin, targetPin, refs, sourceNode, targetNode);
}

function invertPinType(type: IPinType): IPinType {
	return type === IPinType.Input ? IPinType.Output : IPinType.Input;
}

export function doPinsMatch(
	sourcePin: IPin,
	targetPin: IPin,
	refs: { [key: string]: string },
	sourceNode?: INode | ILayer,
	targetNode?: INode | ILayer,
) {
	if (sourceNode?.id.endsWith("-return")) {
		sourcePin.pin_type = invertPinType(sourcePin.pin_type);
	}

	if (sourceNode?.id.endsWith("-input")) {
		sourcePin.pin_type = invertPinType(sourcePin.pin_type);
	}

	if (targetNode?.id.endsWith("-return")) {
		targetPin.pin_type = invertPinType(targetPin.pin_type);
	}

	if (targetNode?.id.endsWith("-input")) {
		targetPin.pin_type = invertPinType(targetPin.pin_type);
	}

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

	if (sourcePin.pin_type === targetPin.pin_type) {
		console.warn(
			`Invalid connection: source and target pins have the same type (${sourcePin.pin_type})`,
		);
		return false;
	}

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
	boardRef?: RefObject<IBoard | undefined>,
	version?: [number, number, number],
) {
	const nodes: any[] = [];
	const edges: any[] = [];
	const cache = new Map<string, [IPin, INode | ILayer, boolean]>();
	const oldNodesMap = new Map<number, any>();
	const oldEdgesMap = new Map<string, any>();

	for (const oldNode of oldNodes ?? []) {
		oldNode.data.boardRef = boardRef;
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
		if (oldNode) {
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
					boardRef: boardRef,
					node: node,
					hash: hash,
					boardId: board.id,
					appId: appId,
					version: version,
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
			if (parentLayer !== currentLayer) {
				if (layer.id === currentLayer) {
					// Build immutable inverted pins for the current layer view and split into input/return inner nodes
					const inputNodePins: { [key: string]: IPin } = {};
					const returnNodePins: { [key: string]: IPin } = {};

					for (const pin of Object.values(layer.pins)) {
						const inverted: IPin = {
							...pin,
							pin_type: invertPinType(pin.pin_type),
						};
						// cache the inverted pin with the layer as owner; visibility = true (we are inside this layer)
						cache.set(inverted.id, [inverted, layer, true]);
						// Pins that become Output feed the -input node; those that become Input feed the -return node
						if (inverted.pin_type === IPinType.Output) {
							inputNodePins[inverted.id] = inverted;
						} else {
							returnNodePins[inverted.id] = inverted;
						}
					}

					nodes.push({
						id: layer.id + "-input",
						type: "layerInnerNode",
						position: {
							x: layer.in_coordinates?.[0],
							y: layer.in_coordinates?.[1],
						},
						zIndex: 19,
						data: {
							label: layer.id,
							boardId: board.id,
							appId: appId,
							boardRef: boardRef,
							type: InnerLayerNodeType.INPUT,
							layer: {
								...layer,
								pins: inputNodePins,
							},
							hash: layer.hash ?? -1,
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
						selected: selected.has(layer.id + "-input"),
					});
					nodes.push({
						id: layer.id + "-return",
						type: "layerInnerNode",
						position: {
							x: layer.out_coordinates?.[0],
							y: layer.out_coordinates?.[1],
						},
						zIndex: 19,
						data: {
							label: layer.id,
							boardId: board.id,
							appId: appId,
							boardRef: boardRef,
							type: InnerLayerNodeType.RETURN,
							layer: {
								...layer,
								pins: returnNodePins,
							},
							hash: layer.hash ?? -1,
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
						selected: selected.has(layer.id + "-return"),
					});
				}

				continue;
			}

			const lookup: Record<string, INode | ILayer> = {};
			if (layer.pins)
				for (const pin of Object.values(layer.pins)) {
					const [_, node] = cache.get(pin.id) ?? [pin.id, layer];
					if (node) lookup[pin.id] = node;
					cache.set(pin.id, [pin, node, true]);
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
					boardRef: boardRef,
					hash: layer.hash ?? -1,
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

	// Helper to resolve inner node id for current layer boundary pins
	const resolveInnerNodeId = (layerId: string, pin: IPin) =>
		// Pins were inverted when entering the current layer view:
		// - boundary Input -> inverted Output -> belongs to `${layerId}-input`
		// - boundary Output -> inverted Input -> belongs to `${layerId}-return`
		pin.pin_type === IPinType.Output ? `${layerId}-input` : `${layerId}-return`;

	const currentLayerRef: ILayer | undefined = board.layers[currentLayer ?? ""];
	for (const [pin, node, visible] of cache.values()) {
		if (pin.connected_to.length === 0) continue;

		for (const connectedTo of pin.connected_to) {
			const [conntectedPin, connectedNode, connectedVisible] =
				cache.get(connectedTo) || [];
			const connectedLayer = board.layers[connectedNode?.layer ?? ""];
			if (!visible && !connectedVisible) continue;
			if (!conntectedPin || !connectedNode) continue;

			if (
				visible !== connectedVisible &&
				(connectedLayer?.parent_id ?? "") !== (currentLayer ?? "")
			) {
				if (!visible && node.layer === currentLayerRef?.parent_id) {
					let coordinates = node.coordinates ?? [0, 0, 0];

					if (currentLayerRef?.nodes[node.id]) {
						coordinates = currentLayerRef.nodes[node.id]?.coordinates ?? [
							0, 0, 0,
						];
					}
				} else if (
					!connectedVisible &&
					connectedNode.layer === currentLayerRef?.parent_id
				) {
					let coordinates = connectedNode.coordinates ?? [0, 0, 0];

					if (currentLayerRef?.nodes[connectedNode.id]) {
						coordinates = currentLayerRef.nodes[connectedNode.id]
							?.coordinates ?? [0, 0, 0];
					}
				}
			}

			const edge = oldEdgesMap.get(`${pin.id}-${connectedTo}`);

			if (
				edge &&
				visible === connectedVisible &&
				edge.data.fromLayer === (node as any).layer &&
				edge.data.toLayer === (connectedNode as any).layer &&
				currentLayer !== (connectedNode as any).layer &&
				currentLayer !== (node as any).layer
			) {
				edges.push(edge);
				continue;
			}

			// Map endpoints:
			// - If the owner is the current layer, route to the inner nodes (-input / -return)
			// - Else, if the owner lives in an active child layer, route to that layer node
			// - Else, route to the node itself
			const sourceNodeId =
				((node as any)?.id ?? "") === (currentLayer ?? "")
					? resolveInnerNodeId(currentLayer!, pin)
					: activeLayer.has((node as any)?.layer ?? "")
						? (node as any).layer
						: (node as any)?.id;

			const targetNodeId =
				((connectedNode as any)?.id ?? "") === (currentLayer ?? "")
					? resolveInnerNodeId(currentLayer!, conntectedPin)
					: activeLayer.has((connectedNode as any)?.layer ?? "")
						? (connectedNode as any).layer
						: (connectedNode as any)?.id;

			if (pin.id && conntectedPin.id)
				edges.push({
					id: `${pin.id}-${conntectedPin.id}`,
					source: sourceNodeId,
					sourceHandle: pin.id,
					zIndex: 18,
					data: {
						fromLayer: (node as any).layer,
						toLayer: (connectedNode as any).layer,
						pathType: connectionMode,
						data_type: pin.data_type,
					},
					animated: pin.data_type !== "Execution",
					reconnectable: true,
					target: targetNodeId,
					targetHandle: conntectedPin.id,
					style: { stroke: typeToColor(pin.data_type) },
					type: pin.data_type === "Execution" ? "execution" : "data",
					data_type: pin.data_type,
					selected: selected.has(`${pin.id}-${connectedTo}`),
				});
			else {
				console.log(`${pin.id}-${connectedTo} edge not created`);
			}
		}
	}

	// Create edges for function references
	for (const node of Object.values(board.nodes)) {
		const nodeLayer = (node.layer ?? "") === "" ? undefined : node.layer;
		if (nodeLayer !== currentLayer) continue;

		if (node.fn_refs?.can_reference_fns && node.fn_refs.fn_refs.length > 0) {
			for (const refNodeId of node.fn_refs.fn_refs) {
				const targetNode = board.nodes[refNodeId];
				if (!targetNode) continue;

				const targetLayer =
					(targetNode.layer ?? "") === "" ? undefined : targetNode.layer;
				if (targetLayer !== currentLayer) continue;

				const sourceHandle = `ref_out_${node.id}`;
				const targetHandle = `ref_in_${refNodeId}`;
				const edgeId = `${sourceHandle}-${targetHandle}`;

				const existingEdge = oldEdgesMap.get(edgeId);

				if (existingEdge) {
					edges.push(existingEdge);
				} else {
					edges.push({
						id: edgeId,
						source: node.id,
						sourceHandle: sourceHandle,
						target: refNodeId,
						targetHandle: targetHandle,
						zIndex: 18,
						data: {
							fromLayer: nodeLayer,
							toLayer: targetLayer,
							isFnRef: true,
							pathType: connectionMode,
						},
						animated: true,
						reconnectable: true,
						style: {
							stroke: "var(--pin-fn-ref)",
						},
						type: "veil",
						selected: selected.has(edgeId),
					});
				}
			}
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
				boardRef: boardRef,
				comment: { ...comment, is_locked: comment.is_locked ?? false },
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
