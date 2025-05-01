"use client";
import { DragOverlay, useDroppable } from "@dnd-kit/core";
import { createId } from "@paralleldrive/cuid2";
import type { UseQueryResult } from "@tanstack/react-query";
import {
	Background,
	BackgroundVariant,
	type Connection,
	Controls,
	type Edge,
	type FinalConnectionState,
	type InternalNode,
	type IsValidConnection,
	MiniMap,
	type OnEdgesChange,
	type OnNodesChange,
	ReactFlow,
	addEdge,
	applyEdgeChanges,
	applyNodeChanges,
	reconnectEdge,
	useEdgesState,
	useNodesState,
	useReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
	ArrowBigLeftDashIcon,
	HistoryIcon,
	NotebookPenIcon,
	PlayCircleIcon,
	Redo2Icon,
	ScrollIcon,
	Undo2Icon,
	VariableIcon,
	XIcon,
} from "lucide-react";
import MiniSearch from "minisearch";
import { useTheme } from "next-themes";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ImperativePanelHandle } from "react-resizable-panels";
import { useLogAggregation } from "../..";
import { CommentNode } from "../../components/flow/comment-node";
import { FlowContextMenu } from "../../components/flow/flow-context-menu";
import { FlowDock } from "../../components/flow/flow-dock";
import { FlowNode } from "../../components/flow/flow-node";
import { Traces } from "../../components/flow/traces";
import {
	Variable,
	VariablesMenu,
} from "../../components/flow/variables/variables-menu";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "../../components/ui/resizable";
import { useInvalidateInvoke, useInvoke } from "../../hooks/use-invoke";
import {
	type IGenericCommand,
	type ILogMetadata,
	addNodeCommand,
	connectPinsCommand,
	disconnectPinsCommand,
	moveNodeCommand,
	removeCommentCommand,
	removeNodeCommand,
	upsertCommentCommand,
} from "../../lib";
import {
	handleCopy,
	handlePaste,
	isValidConnection,
	parseBoard,
} from "../../lib/flow-board";
import { toastError, toastSuccess } from "../../lib/messages";
import {
	type IComment,
	ICommentType,
	IExecutionStage,
	ILogLevel,
	type IVariable,
} from "../../lib/schema/flow/board";
import { type INode, IVariableType } from "../../lib/schema/flow/node";
import type { IPin } from "../../lib/schema/flow/pin";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { useBackend } from "../../state/backend-state";
import { useFlowBoardParentState } from "../../state/flow-board-parent-state";
import { useRunExecutionStore } from "../../state/run-execution-state";
import {
	Button,
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	Input,
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Separator,
	Textarea,
} from "../ui";
import { useUndoRedo } from "./flow-history";
import { FlowRuns } from "./flow-runs";
import { LayerNode } from "./layer-node";
export function FlowBoard({
	appId,
	boardId,
}: Readonly<{ appId: string; boardId: string }>) {
	const invalidate = useInvalidateInvoke();
	const { pushCommand, pushCommands, redo, undo } = useUndoRedo(appId, boardId);
	const router = useRouter();
	const backend = useBackend();
	const selected = useRef(new Set<string>());
	const edgeReconnectSuccessful = useRef(true);
	const { isOver, setNodeRef, active } = useDroppable({ id: "flow" });
	const parentRegister = useFlowBoardParentState();
	const { refetchLogs, setCurrentMetadata } = useLogAggregation();
	const flowRef = useRef<any>(null);

	const flowPanelRef = useRef<ImperativePanelHandle>(null);
	const logPanelRef = useRef<ImperativePanelHandle>(null);
	const varPanelRef = useRef<ImperativePanelHandle>(null);
	const runsPanelRef = useRef<ImperativePanelHandle>(null);

	const { resolvedTheme } = useTheme();

	const catalog: UseQueryResult<INode[]> = useInvoke(backend.getCatalog, []);
	const board = useInvoke(backend.getBoard, [appId, boardId], boardId !== "");
	const openBoards = useInvoke(backend.getOpenBoards, []);
	const currentProfile = useInvoke(backend.getSettingsProfile, []);
	const { addRun, removeRun, pushUpdate } = useRunExecutionStore();

	const [nodes, setNodes] = useNodesState<any>([]);
	const [edges, setEdges] = useEdgesState<any>([]);
	const [filteredNodes, setFilteredNodes] = useState<INode[]>([]);
	const [droppedPin, setDroppedPin] = useState<IPin | undefined>(undefined);
	const [clickPosition, setClickPosition] = useState({ x: 0, y: 0 });
	const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });
	const [pinCache, setPinCache] = useState<Map<string, [IPin, INode]>>(
		new Map(),
	);
	const [editBoard, setEditBoard] = useState(false);
	const [boardMeta, setBoardMeta] = useState({
		name: "",
		description: "",
		stage: "Dev",
		logLevel: "Debug",
	});
	const [layer, setLayer] = useState<string | undefined>();
	const colorMode = useMemo(
		() => (resolvedTheme === "dark" ? "dark" : "light"),
		[resolvedTheme],
	);

	const executeCommand = useCallback(
		async (command: IGenericCommand, append = false): Promise<any> => {
			const result = await backend.executeCommand(appId, boardId, command);
			await pushCommand(result, append);
			await board.refetch();
			return result;
		},
		[board.refetch],
	);

	const executeCommands = useCallback(
		async (commands: IGenericCommand[]) => {
			if (commands.length === 0) return;
			const result = await backend.executeCommands(appId, boardId, commands);
			await pushCommands(result);
			await board.refetch();
			return result;
		},
		[board.refetch],
	);

	useEffect(() => {
		if (!logPanelRef.current) return;

		logPanelRef.current.expand();
		const size = logPanelRef.current.getSize();

		if (size < 10) logPanelRef.current.resize(45);
	}, [logPanelRef.current]);

	function toggleVars() {
		if (!varPanelRef.current) return;
		const isCollapsed = varPanelRef.current.isCollapsed();
		isCollapsed ? varPanelRef.current.expand() : varPanelRef.current.collapse();

		if (!isCollapsed) {
			return;
		}

		const size = varPanelRef.current.getSize();
		if (size < 10) varPanelRef.current.resize(20);
	}

	function toggleRunHistory() {
		if (!runsPanelRef.current) return;
		const isCollapsed = runsPanelRef.current.isCollapsed();
		isCollapsed
			? runsPanelRef.current.expand()
			: runsPanelRef.current.collapse();

		if (!isCollapsed) {
			return;
		}

		const size = runsPanelRef.current.getSize();
		if (size < 10) runsPanelRef.current.resize(20);
	}

	const executeBoard = useCallback(
		async (node: INode, payload?: object) => {
			let added = false;
			console.log(appId);
			const runMeta: ILogMetadata | undefined = await backend.executeBoard(
				appId,
				boardId,
				{
					id: node.id,
					payload: payload,
				},
				(update) => {
					const runUpdates = update
						.filter((item) => item.event_type.startsWith("run:"))
						.map((item) => item.payload);
					if (runUpdates.length === 0) return;
					const firstItem = runUpdates[0];
					if (!added) {
						addRun(firstItem.run_id, boardId, [node.id]);
						added = true;
					}

					pushUpdate(firstItem.run_id, runUpdates);
				},
			);
			if (!runMeta) {
				toastError(
					"Failed to execute board",
					<PlayCircleIcon className="w-4 h-4" />,
				);
				return;
			}
			removeRun(runMeta.run_id);
			await backend.finalizeRun(appId, runMeta.run_id);
			await refetchLogs(backend);
			setCurrentMetadata(runMeta);
		},
		[appId, boardId, backend],
	);

	const handlePasteCB = useCallback(
		async (event: ClipboardEvent) => {
			const currentCursorPosition = screenToFlowPosition({
				x: mousePosition.x,
				y: mousePosition.y,
			});
			await handlePaste(event, currentCursorPosition, boardId, executeCommand);
		},
		[boardId, mousePosition, executeCommand],
	);

	const handleCopyCB = useCallback(
		(event: ClipboardEvent) => {
			handleCopy(nodes, event);
		},
		[nodes],
	);

	const placeNodeShortcut = useCallback(
		async (node: INode) => {
			await placeNode(node, {
				x: mousePosition.x,
				y: mousePosition.y,
			});
		},
		[mousePosition],
	);

	const shortcutHandler = useCallback(
		async (event: KeyboardEvent) => {
			// Undo
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "z" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				const stack = await undo();
				if (stack) await backend.undoBoard(appId, boardId, stack);
				toastSuccess("Undo", <Undo2Icon className="w-4 h-4" />);
				await board.refetch();
				return;
			}

			// Redo
			if ((event.metaKey || event.ctrlKey) && event.key === "y") {
				event.preventDefault();
				event.stopPropagation();
				const stack = await redo();
				if (stack) await backend.redoBoard(appId, boardId, stack);
				toastSuccess("Redo", <Redo2Icon className="w-4 h-4" />);
				await board.refetch();
			}

			// Place Branch
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "b" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				const node = catalog.data?.find(
					(node) => node.name === "control_branch",
				);
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}

			// Place For Each
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "f" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				const node = catalog.data?.find(
					(node) => node.name === "control_for_each",
				);
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}

			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "p" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				const node = catalog.data?.find((node) => node.name === "log_info");
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}

			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "s" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				const node = catalog.data?.find((node) => node.name === "reroute");
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}
		},
		[boardId, board, backend],
	);

	const placeNode = useCallback(
		async (node: INode, position?: { x: number; y: number }) => {
			const refs = board.data?.refs ?? {};
			const location = screenToFlowPosition({
				x: position?.x ?? clickPosition.x,
				y: position?.y ?? clickPosition.y,
			});
			const result = addNodeCommand({
				node: { ...node, coordinates: [location.x, location.y, 0] },
			});

			await executeCommand(result.command);
			const new_node = result.node;
			if (droppedPin) {
				const pinType = droppedPin.pin_type === "Input" ? "Output" : "Input";
				const pinValueType = droppedPin.value_type;
				const pinDataType = droppedPin.data_type;
				const schema = refs?.[droppedPin.schema ?? ""] ?? droppedPin.schema;
				const options = droppedPin.options;

				const pin = Object.values(new_node.pins).find((pin) => {
					if (typeof schema === "string" || typeof pin.schema === "string") {
						const pinSchema = refs?.[pin.schema ?? ""] ?? pin.schema;
						if (
							(pin.options?.enforce_schema || options?.enforce_schema) &&
							schema !== pinSchema &&
							pin.data_type !== IVariableType.Generic
						)
							return false;
					}
					if (pin.pin_type !== pinType) return false;
					if (pin.value_type !== pinValueType) {
						if (
							pinDataType !== IVariableType.Generic &&
							pin.data_type !== IVariableType.Generic
						)
							return false;
						if (
							options?.enforce_generic_value_type ||
							pin.options?.enforce_generic_value_type
						)
							return false;
					}
					if (
						pin.data_type === IVariableType.Generic &&
						pinDataType !== IVariableType.Execution
					)
						return true;
					if (
						pinDataType === IVariableType.Generic &&
						pin.data_type !== IVariableType.Execution
					)
						return true;
					return pin.data_type === pinDataType;
				});
				const [sourcePin, sourceNode] = pinCache.get(droppedPin.id) || [];
				if (!sourcePin || !sourceNode) return;
				if (!pin) return;

				const command = connectPinsCommand({
					from_node:
						droppedPin.pin_type === "Output" ? sourceNode.id : new_node.id,
					from_pin: droppedPin.pin_type === "Output" ? sourcePin.id : pin?.id,
					to_node:
						droppedPin.pin_type === "Input" ? sourceNode.id : new_node.id,
					to_pin: droppedPin.pin_type === "Input" ? sourcePin.id : pin?.id,
				});

				await executeCommand(command);
			}
		},
		[clickPosition, boardId, droppedPin, board.data?.refs],
	);

	const handleDrop = useCallback(
		async (event: any) => {
			const variable: IVariable = event.detail.variable;
			const operation: "set" | "get" = event.detail.operation;
			const screenPosition = event.detail.screenPosition;
			const getVarNode = catalog.data?.find(
				(node) => node.name === `variable_${operation}`,
			);
			if (!getVarNode) return console.dir(catalog.data);

			const varRefPin = Object.values(getVarNode.pins).find(
				(pin) => pin.name === "var_ref",
			);
			if (!varRefPin) return;

			varRefPin.default_value = convertJsonToUint8Array(variable.id);
			getVarNode.pins[varRefPin.id] = varRefPin;

			placeNode(getVarNode, {
				x: screenPosition.x,
				y: screenPosition.y,
			});
		},
		[catalog.data, clickPosition, boardId, droppedPin],
	);

	useEffect(() => {
		document.addEventListener("copy", handleCopyCB);
		document.addEventListener("paste", handlePasteCB);

		return () => {
			document.removeEventListener("copy", handleCopyCB);
			document.removeEventListener("paste", handlePasteCB);
		};
	}, [nodes]);

	useEffect(() => {
		document.addEventListener("keydown", shortcutHandler);
		return () => {
			document.removeEventListener("keydown", shortcutHandler);
		};
	}, [shortcutHandler]);

	useEffect(() => {
		document.addEventListener("flow-drop", handleDrop);
		return () => {
			document.removeEventListener("flow-drop", handleDrop);
		};
	}, [handleDrop]);

	useEffect(() => {
		document.addEventListener("mousemove", (event) => {
			setMousePosition({ x: event.clientX, y: event.clientY });
		});

		return () => {
			document.removeEventListener("mousemove", (event) => {
				setMousePosition({ x: event.clientX, y: event.clientY });
			});
		};
	}, []);

	useEffect(() => {
		if (!board.data) return;

		const parsed = parseBoard(
			board.data,
			appId,
			executeBoard,
			executeCommand,
			selected.current,
			currentProfile.data?.flow_settings.connection_mode,
			nodes,
			edges,
			layer,
		);

		setNodes(parsed.nodes);
		setEdges(parsed.edges);
		setPinCache(new Map(parsed.cache));
		setBoardMeta({
			name: board.data.name,
			description: board.data.description,
			stage: board.data.stage,
			logLevel: board.data.log_level,
		});
	}, [board.data, layer]);

	const miniSearch = useMemo(
		() =>
			new MiniSearch({
				fields: [
					"name",
					"friendly_name",
					"category",
					"description",
					"pin_in_names",
					"pin_out_names",
				],
				storeFields: ["id"],
			}),
		[],
	);

	useEffect(() => {
		if (!catalog.data) return;
		miniSearch.removeAll();
		miniSearch.addAll(
			catalog.data.map((node) => ({
				...node,
				pin_in_names: Object.values(node.pins)
					.filter((pin) => pin.pin_type === "Input")
					.map((pin) => pin.name)
					.join(" "),
				pin_out_names: Object.values(node.pins)
					.filter((pin) => pin.pin_type === "Output")
					.map((pin) => pin.name)
					.join(" "),
			})),
		);
		setFilteredNodes(catalog.data);
	}, [catalog.data]);

	const nodeTypes = useMemo(
		() => ({
			flowNode: FlowNode,
			commentNode: CommentNode,
			layerNode: LayerNode,
			node: FlowNode,
		}),
		[],
	);
	const { screenToFlowPosition } = useReactFlow();

	const onConnect = useCallback(
		(params: any) =>
			setEdges((eds) => {
				const [sourcePin, sourceNode] = pinCache.get(params.sourceHandle) || [];
				const [targetPin, targetNode] = pinCache.get(params.targetHandle) || [];

				if (!sourcePin || !targetPin) return eds;
				if (!sourceNode || !targetNode) return eds;

				const command = connectPinsCommand({
					from_node: sourceNode.id,
					from_pin: sourcePin.id,
					to_node: targetNode.id,
					to_pin: targetPin.id,
				});

				executeCommand(command);

				return addEdge(params, eds);
			}),
		[setEdges, pinCache, boardId],
	);

	const onConnectEnd = useCallback(
		(
			event: MouseEvent | TouchEvent,
			connectionState: FinalConnectionState<InternalNode>,
		) => {
			// when a connection is dropped on the pane it's not valid
			if (!connectionState.isValid) {
				// we need to remove the wrapper bounds, in order to get the correct position

				const { clientX, clientY } =
					"changedTouches" in event ? event.changedTouches[0] : event;

				const handle = connectionState.fromHandle;
				if (handle?.id) {
					const [pin, _node] = pinCache.get(handle.id) || [];
					setDroppedPin(pin);
				}

				const contextMenuEvent = new MouseEvent("contextmenu", {
					bubbles: true,
					cancelable: true,
					view: window,
					clientX,
					clientY,
				});

				flowRef.current?.dispatchEvent(contextMenuEvent);
			}
		},
		[pinCache],
	);

	const onNodesChangeIntercept: OnNodesChange = useCallback(
		(changes: any[]) =>
			setNodes((nds) => {
				if (!changes) return applyNodeChanges(changes, nds);

				const selectChanges = changes.filter(
					(change: any) => change.type === "select",
				);
				for (const change of selectChanges) {
					const selectedId = change.id;

					if (change.selected) selected.current.add(selectedId);
					if (!change.selected) selected.current.delete(selectedId);
				}

				const removeChanges = changes.filter(
					(change: any) => change.type === "remove",
				);
				executeCommands(
					removeChanges
						.map((change) => {
							const foundNode = Object.values(board.data?.nodes || {}).find(
								(node) => node.id === change.id,
							);
							if (foundNode) {
								return removeNodeCommand({
									node: foundNode,
									connected_nodes: [],
								});
							}
							const foundComment = Object.values(
								board.data?.comments || {},
							).find((comment) => comment.id === change.id);
							if (foundComment) {
								return removeCommentCommand({
									comment: foundComment,
								});
							}
							return undefined;
						})
						.filter((command) => command !== undefined) as any[],
				);

				return applyNodeChanges(changes, nds);
			}),
		[setNodes, board.data, boardId, executeCommands],
	);

	const onEdgesChange: OnEdgesChange = useCallback(
		(changes: any[]) =>
			setEdges((eds) => {
				if (!changes || changes.length === 0)
					return applyEdgeChanges(changes, eds);

				const selectChanges = changes.filter(
					(change: any) => change.type === "select",
				);
				for (const change of selectChanges) {
					const selectedId = change.id;
					const selectedEdge: any = eds.find((edge) => edge.id === selectedId);

					if (change.selected) selected.current.add(selectedId);
					if (!change.selected) selected.current.delete(selectedId);

					if (selectedEdge.data_type !== "Execution")
						eds = eds.map((edge) =>
							edge.id === selectedId
								? { ...edge, animated: !change.selected }
								: edge,
						);
				}

				const removeChanges = changes.filter(
					(change: any) => change.type === "remove",
				);
				executeCommands(
					removeChanges
						.map((change: any) => {
							const selectedId = change.id;
							const [fromPinId, toPinId] = selectedId.split("-");
							const [fromPin, fromNode] = pinCache.get(fromPinId) || [];
							const [toPin, toNode] = pinCache.get(toPinId) || [];

							if (!fromPin || !toPin) return undefined;
							if (!fromNode || !toNode) return undefined;

							return {
								command: "disconnect_pins",
								args: {
									boardId: boardId,
									fromNode: fromNode.id,
									fromPin: fromPin.id,
									toNode: toNode.id,
									toPin: toPin.id,
								},
							};
						})
						.filter((command: any) => command !== undefined) as any[],
				);

				return applyEdgeChanges(changes, eds);
			}),
		[setEdges, board.data, boardId],
	);

	const onReconnectStart = useCallback(() => {
		edgeReconnectSuccessful.current = false;
	}, []);

	const onReconnect = useCallback(
		async (oldEdge: any, newConnection: Connection) => {
			// Check if the edge is actually being moved
			const new_id = `${newConnection.sourceHandle}-${newConnection.targetHandle}`;
			if (oldEdge.id === new_id) return;

			const disconnectCommand = disconnectPinsCommand({
				from_node: oldEdge.source,
				from_pin: oldEdge.sourceHandle,
				to_node: oldEdge.target,
				to_pin: oldEdge.targetHandle,
			});

			await executeCommand(disconnectCommand);

			if (!newConnection.targetHandle || !newConnection.sourceHandle) return;

			const connectCommand = connectPinsCommand({
				from_node: newConnection.source,
				from_pin: newConnection.sourceHandle,
				to_node: newConnection.target,
				to_pin: newConnection.targetHandle,
			});

			await executeCommand(connectCommand, true);

			edgeReconnectSuccessful.current = true;
			setEdges((els) => reconnectEdge(oldEdge, newConnection, els));
		},
		[boardId, setEdges],
	);

	const onReconnectEnd = useCallback(
		async (event: any, edge: any) => {
			if (!edgeReconnectSuccessful.current) {
				const { source, target, sourceHandle, targetHandle } = edge;
				const command = disconnectPinsCommand({
					from_node: source,
					from_pin: sourceHandle,
					to_node: target,
					to_pin: targetHandle,
				});
				await executeCommand(command);
				setEdges((eds) => eds.filter((e) => e.id !== edge.id));
			}

			edgeReconnectSuccessful.current = true;
		},
		[boardId, setEdges],
	);

	const onContextMenuCB = useCallback((event: any) => {
		setClickPosition({ x: event.clientX, y: event.clientY });
	}, []);

	const onNodeDragStop = useCallback(
		async (event: any, node: any, nodes: any) => {
			const commands: IGenericCommand[] = [];
			for await (const node of nodes) {
				console.log(
					`Moving node ${node.id} to ${node.position.x}, ${node.position.y}`,
				);
				const comment = Object.values(board.data?.comments || {}).find(
					(comment) => comment.id === node.id,
				);
				if (comment) {
					const command = upsertCommentCommand({
						comment: {
							...comment,
							coordinates: [node.position.x, node.position.y, 0],
						},
					});
					commands.push(command);
				}

				if (!comment) {
					const command = moveNodeCommand({
						node_id: node.id,
						to_coordinates: [node.position.x, node.position.y, 0],
					});

					commands.push(command);
				}
			}
			await executeCommands(commands);
		},
		[boardId, executeCommands],
	);

	const isValidConnectionCB = useCallback(
		(connection: Edge | Connection) => {
			return isValidConnection(connection, pinCache, board.data?.refs ?? {});
		},
		[pinCache, board.data?.refs],
	) as IsValidConnection<Edge>;

	return (
		<div className="min-h-dvh h-dvh max-h-dvh w-full flex-1 flex-grow">
			<div className="flex items-center justify-center absolute translate-x-[-50%] left-[50dvw] top-5 z-40">
				<Dialog
					open={editBoard}
					onOpenChange={async (open) => {
						if (open) return;
						await backend.updateBoardMeta(
							appId,
							boardId,
							boardMeta.name,
							boardMeta.description,
							boardMeta.logLevel as ILogLevel,
							boardMeta.stage as IExecutionStage,
						);
						await openBoards.refetch();
						setEditBoard(false);
					}}
				>
					<DialogContent>
						<DialogHeader>
							<DialogTitle>Edit Board Metadata</DialogTitle>
							<DialogDescription>
								Give your Board a name and Description for future use!
							</DialogDescription>
						</DialogHeader>
						<Separator />
						<div className="grid w-full max-w-sm items-center gap-1.5">
							<Label htmlFor="name">Name</Label>
							<Input
								value={boardMeta.name}
								onChange={(e) =>
									setBoardMeta((old) => ({ ...old, name: e.target.value }))
								}
								type="text"
								id="name"
								placeholder="Name"
							/>
						</div>
						<div className="grid w-full max-w-sm items-center gap-1.5">
							<Label htmlFor="description">Description</Label>
							<Textarea
								value={boardMeta.description}
								onChange={(e) =>
									setBoardMeta((old) => ({
										...old,
										description: e.target.value,
									}))
								}
								id="description"
								placeholder="Description"
							/>
						</div>
						<div className="grid w-full max-w-sm items-center gap-1.5">
							<Label htmlFor="stage">Stage</Label>
							<Select
								value={boardMeta.stage}
								onValueChange={(e) =>
									setBoardMeta((old) => ({
										...old,
										stage: e as IExecutionStage,
									}))
								}
							>
								<SelectTrigger id="stage" className="w-full max-w-sm">
									<SelectValue placeholder="Stage" />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value={IExecutionStage.Dev}>
										Development
									</SelectItem>
									<SelectItem value={IExecutionStage.Int}>
										Integration
									</SelectItem>
									<SelectItem value={IExecutionStage.QA}>QA</SelectItem>
									<SelectItem value={IExecutionStage.PreProd}>
										Pre-Production
									</SelectItem>
									<SelectItem value={IExecutionStage.Prod}>
										Production
									</SelectItem>
								</SelectContent>
							</Select>
						</div>
						<div className="grid w-full max-w-sm items-center gap-1.5">
							<Label htmlFor="stage">Log Level</Label>
							<Select
								value={boardMeta.logLevel}
								onValueChange={(e) =>
									setBoardMeta((old) => ({ ...old, logLevel: e as ILogLevel }))
								}
							>
								<SelectTrigger id="stage" className="w-full max-w-sm">
									<SelectValue placeholder="Stage" />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value={ILogLevel.Debug}>Debug</SelectItem>
									<SelectItem value={ILogLevel.Info}>Info</SelectItem>
									<SelectItem value={ILogLevel.Warn}>Warning</SelectItem>
									<SelectItem value={ILogLevel.Error}>Error</SelectItem>
									<SelectItem value={ILogLevel.Fatal}>Fatal</SelectItem>
								</SelectContent>
							</Select>
						</div>
					</DialogContent>
				</Dialog>
				<FlowDock
					items={[
						...(typeof parentRegister.boardParents[boardId] === "string"
							? [
									{
										icon: <ArrowBigLeftDashIcon />,
										title: "Back",
										onClick: async () => {
											const urlWithQuery = parentRegister.boardParents[boardId];
											router.push(urlWithQuery);
										},
									},
								]
							: []),
						{
							icon: <VariableIcon />,
							title: "Variables",
							onClick: async () => {
								toggleVars();
							},
						},
						{
							icon: <NotebookPenIcon />,
							title: "Rename Board",
							onClick: async () => {
								setEditBoard(true);
							},
						},
						{
							icon: <HistoryIcon />,
							title: "Run History",
							onClick: async () => {
								toggleRunHistory();
							},
						},
						// ...(currentRun
						// 	? [
						// 			{
						// 				icon: <ScrollIcon />,
						// 				title: "Logs",
						// 				onClick: async () => {
						// 					setTraces((old) =>
						// 						old
						// 							? undefined
						// 							: { node: undefined, traces: currentRun.traces },
						// 					);
						// 				},
						// 			},
						// 		]
						// 	: ([] as any)),
					]}
				/>
			</div>
			<ResizablePanelGroup direction="horizontal">
				<ResizablePanel
					autoSave="flow-variables"
					defaultSize={0}
					collapsible={true}
					collapsedSize={0}
					ref={varPanelRef}
				>
					{board.data && (
						<VariablesMenu board={board.data} executeCommand={executeCommand} />
					)}
				</ResizablePanel>
				<ResizableHandle withHandle />
				<ResizablePanel autoSave="flow-main-container">
					<ResizablePanelGroup direction="vertical">
						<ResizablePanel autoSave="flow-main" ref={flowPanelRef}>
							<FlowContextMenu
								droppedPin={droppedPin}
								onCommentPlace={async () => {
									const location = screenToFlowPosition({
										x: clickPosition.x,
										y: clickPosition.y,
									});
									const new_comment: IComment = {
										comment_type: ICommentType.Text,
										content: "New Comment",
										coordinates: [location.x, location.y, 0],
										id: createId(),
										timestamp: {
											nanos_since_epoch: 0,
											secs_since_epoch: 0,
										},
										author: "anonymous",
									};

									const command = upsertCommentCommand({
										comment: new_comment,
									});

									await executeCommand(command);
								}}
								refs={board.data?.refs || {}}
								onClose={() => setDroppedPin(undefined)}
								nodes={filteredNodes.toSorted((a, b) =>
									a.friendly_name.localeCompare(b.friendly_name),
								)}
								onNodePlace={async (node) => {
									await placeNode(node);
								}}
								onFilterSearch={(filter) => {
									if (filter === "") {
										setFilteredNodes(catalog.data || []);
										return;
									}

									const search = miniSearch.search(filter, {
										prefix: true,
										fuzzy: 0.2,
										boost: {
											name: 1,
											friendly_name: 2,
											category: 0.5,
											description: 0.5,
											pin_in_names: 0.5,
											pin_out_names: 0.5,
										},
									});

									const ids: Set<string> = new Set(
										search.map((node: any) => node.id),
									);
									setFilteredNodes(
										catalog.data?.filter((node) => ids.has(node.id)) || [],
									);
								}}
							>
								<div
									className={`w-full h-full relative ${isOver && "border-green-400 border-2 z-10"}`}
									ref={setNodeRef}
								>
									<div className="absolute top-0 left-0 right-0 p-1 z-40 flex flex-row items-center gap-1">
										{openBoards.data?.map(([appId, boardLoadId, boardName]) => (
											<Link
												key={boardLoadId}
												className={`flex flex-row items-center gap-2 border p-1 px-2 bg-background rounded-md hover:bg-card ${boardLoadId === boardId ? "bg-card" : ""}`}
												href={`/flow?id=${boardLoadId}`}
											>
												<small>{boardName}</small>
												<Button
													size={"icon"}
													className="w-4 h-4"
													onClick={async (e) => {
														e.preventDefault();
														e.stopPropagation();
														await backend.closeBoard(boardLoadId);
														if (boardLoadId === boardId) {
															const nextBoard = openBoards.data?.find(
																([id]) => id !== boardId,
															);
															if (nextBoard) {
																await openBoards.refetch();
																router.push(
																	`/flow?id=${nextBoard[1]}&app=${nextBoard[0]}`,
																);
																return;
															}
															router.push("/");
															await openBoards.refetch();
														}
													}}
												>
													<XIcon className="w-3 h-3" />
												</Button>
											</Link>
										))}
									</div>
									<ReactFlow
										suppressHydrationWarning
										onContextMenu={onContextMenuCB}
										ref={flowRef}
										colorMode={colorMode}
										nodes={nodes}
										nodeTypes={nodeTypes}
										edges={edges}
										onNodesChange={onNodesChangeIntercept}
										onEdgesChange={onEdgesChange}
										onNodeDragStop={onNodeDragStop}
										isValidConnection={isValidConnectionCB}
										onConnect={onConnect}
										onReconnect={onReconnect}
										onReconnectStart={onReconnectStart}
										// onEdgeDoubleClick={(e, edge) => {
										// 	console.dir({e, edge})
										// }}
										onReconnectEnd={onReconnectEnd}
										onConnectEnd={onConnectEnd}
										fitView
										proOptions={{ hideAttribution: true }}
									>
										<Controls />
										<MiniMap />
										<Background
											variant={BackgroundVariant.Dots}
											gap={12}
											size={1}
										/>
									</ReactFlow>
									<DragOverlay
										dropAnimation={{
											duration: 500,
											easing: "cubic-bezier(0.18, 0.67, 0.6, 1.22)",
										}}
									>
										{(active?.data?.current as IVariable)?.id && (
											<Variable
												variable={active?.data?.current as IVariable}
												preview
												onVariableChange={() => {}}
												onVariableDeleted={() => {}}
											/>
										)}
									</DragOverlay>
								</div>
							</FlowContextMenu>
						</ResizablePanel>
						<ResizableHandle withHandle />
						<ResizablePanel
							hidden={true}
							ref={logPanelRef}
							collapsible={true}
							autoSave="flow-log"
						>
							{/* {traces && currentRun && (
								<Traces
									node={traces.node}
									result={currentRun}
									traces={traces.traces}
									onOpenChange={(open) => {
										if (!open) setTraces(undefined);
									}}
								/>
							)} */}
						</ResizablePanel>
					</ResizablePanelGroup>
				</ResizablePanel>
				<ResizableHandle withHandle />
				<ResizablePanel
					autoSave="flow-log"
					defaultSize={0}
					collapsible={true}
					collapsedSize={0}
					ref={runsPanelRef}
				>
					{board.data && (
						<FlowRuns
							executeBoard={executeBoard}
							nodes={board.data.nodes}
							appId={appId}
							boardId={boardId}
						/>
					)}
				</ResizablePanel>
			</ResizablePanelGroup>
		</div>
	);
}
