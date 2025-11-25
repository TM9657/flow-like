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
	type Node,
	type OnEdgesChange,
	type OnNodesChange,
	type OnSelectionChangeFunc,
	ReactFlow,
	type ReactFlowInstance,
	addEdge,
	applyEdgeChanges,
	applyNodeChanges,
	reconnectEdge,
	useEdgesState,
	useKeyPress,
	useNodesState,
	useReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
	ArrowBigLeftDashIcon,
	HistoryIcon,
	NotebookPenIcon,
	PlayCircleIcon,
	ScrollIcon,
	SquareChevronUpIcon,
	VariableIcon,
	WifiIcon,
	WifiOffIcon,
	XIcon,
} from "lucide-react";
import { useTheme } from "next-themes";
import { useRouter } from "next/navigation";
import {
	type ReactElement,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import type { ImperativePanelHandle } from "react-resizable-panels";
import {
	Button,
	Sheet,
	SheetContent,
	SheetHeader,
	SheetTitle,
	useHub,
	useLogAggregation,
	useMobileHeader,
} from "../..";
import { CommentNode } from "../../components/flow/comment-node";
import { FlowContextMenu } from "../../components/flow/flow-context-menu";
import { FlowDock } from "../../components/flow/flow-dock";
import { FlowNode } from "../../components/flow/flow-node";
import {
	FlowNodeInfoOverlay,
	type FlowNodeInfoOverlayHandle,
} from "../../components/flow/flow-node/flow-node-info-overlay";
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
import { useCommandExecution } from "../../hooks/use-command-execution";
import { useFlowPanels } from "../../hooks/use-flow-panels";
import { useInvoke } from "../../hooks/use-invoke";
import { useKeyboardShortcuts } from "../../hooks/use-keyboard-shortcuts";
import { useLayerNavigation } from "../../hooks/use-layer-navigation";
import { useRealtimeCollaboration } from "../../hooks/use-realtime-collaboration";
import { useViewportManager } from "../../hooks/use-viewport-manager";
import {
	type IGenericCommand,
	type ILogMetadata,
	IPinType,
	IValueType,
	addNodeCommand,
	connectPinsCommand,
	disconnectPinsCommand,
	moveNodeCommand,
	removeCommentCommand,
	removeNodeCommand,
	removeVariableCommand,
	updateNodeCommand,
	upsertCommentCommand,
	upsertLayerCommand,
	upsertVariableCommand,
} from "../../lib";
import {
	handleConnection,
	handleEdgesChange,
	handleNodesChange,
	handlePlaceNode,
	handlePlacePlaceholder,
} from "../../lib/flow-board-helpers";
import {
	handleCopy,
	handlePaste,
	hexToRgba,
	isValidConnection,
	parseBoard,
} from "../../lib/flow-board-utils";
import { toastError } from "../../lib/messages";
import {
	type IBoard,
	type IComment,
	ICommentType,
	ILayerType,
	type IVariable,
} from "../../lib/schema/flow/board";
import { type INode, IVariableType } from "../../lib/schema/flow/node";
import type { IPin } from "../../lib/schema/flow/pin";
import type { ILayer } from "../../lib/schema/flow/run";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { useBackend } from "../../state/backend-state";
import { useFlowBoardParentState } from "../../state/flow-board-parent-state";
import { useRunExecutionStore } from "../../state/run-execution-state";
import { BoardMeta } from "./board-meta";
import {
	type BoardCommand,
	FlowCopilot,
	type Suggestion,
} from "./flow-copilot";
import { FlowCursors } from "./flow-cursors";
import { FlowDataEdge } from "./flow-data-edge";
import { FlowExecutionEdge } from "./flow-execution-edge";
import { useUndoRedo } from "./flow-history";
import { FlowLayerIndicators } from "./flow-layer-indicators";
import { PinEditModal } from "./flow-pin/edit-modal";
import { FlowRuns } from "./flow-runs";
import { FlowVeilEdge } from "./flow-veil-edge";
import { LayerInnerNode } from "./layer-inner-node";
import { LayerNode } from "./layer-node";

export function FlowBoard({
	appId,
	boardId,
	nodeId,
	initialVersion,
}: Readonly<{
	appId: string;
	boardId: string;
	nodeId?: string;
	initialVersion?: [number, number, number];
}>) {
	const { pushCommand, pushCommands, redo, undo } = useUndoRedo(appId, boardId);
	const router = useRouter();
	const backend = useBackend();
	const selected = useRef(new Set<string>());
	const hub = useHub();
	const edgeReconnectSuccessful = useRef(true);
	const { isOver, setNodeRef, active } = useDroppable({ id: "flow" });
	const parentRegister = useFlowBoardParentState();
	const { refetchLogs, setCurrentMetadata, currentMetadata } =
		useLogAggregation();
	const flowRef = useRef<any>(null);
	const [version, setVersion] = useState<[number, number, number] | undefined>(
		initialVersion,
	);
	const [initialized, setInitialized] = useState(false);
	const flowPanelRef = useRef<ImperativePanelHandle>(null);
	const logPanelRef = useRef<ImperativePanelHandle>(null);
	const varPanelRef = useRef<ImperativePanelHandle>(null);
	const runsPanelRef = useRef<ImperativePanelHandle>(null);
	const nodeInfoOverlayRef = useRef<FlowNodeInfoOverlayHandle>(null);

	const shiftPressed = useKeyPress("Shift");

	const { resolvedTheme } = useTheme();

	const catalog: UseQueryResult<INode[]> = useInvoke(
		backend.boardState.getCatalog,
		backend.boardState,
		[],
	);
	const board = useInvoke(
		backend.boardState.getBoard,
		backend.boardState,
		[appId, boardId, version],
		boardId !== "",
	);
	const boardRef = useRef<IBoard | undefined>(undefined);
	const currentProfile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);
	const { addRun, removeRun, pushUpdate } = useRunExecutionStore();
	const { screenToFlowPosition, getViewport, setViewport, fitView } =
		useReactFlow();

	const [nodes, setNodes] = useNodesState<any>([]);
	const [edges, setEdges] = useEdgesState<any>([]);
	const [droppedPin, setDroppedPin] = useState<IPin | undefined>(undefined);
	const [clickPosition, setClickPosition] = useState({ x: 0, y: 0 });
	const deletingNodesRef = useRef<Set<string>>(new Set());
	const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });
	const [pinCache, setPinCache] = useState<
		Map<string, [IPin, INode | ILayer, boolean]>
	>(new Map());
	const [editBoard, setEditBoard] = useState(false);
	const [currentLayer, setCurrentLayer] = useState<string | undefined>();
	const [layerPath, setLayerPath] = useState<string | undefined>();
	const colorMode = useMemo(
		() => (resolvedTheme === "dark" ? "dark" : "light"),
		[resolvedTheme],
	);

	const { update: updateHeader } = useMobileHeader();

	useEffect(() => {
		const left: ReactElement[] = [];
		const right: ReactElement[] = [];

		if (
			typeof parentRegister.boardParents[boardId] === "string" &&
			!currentLayer
		) {
			left.push(
				<Button
					variant={"default"}
					size={"icon"}
					onClick={async () => {
						const urlWithQuery = parentRegister.boardParents[boardId];
						router.push(urlWithQuery);
					}}
				>
					<ArrowBigLeftDashIcon />
				</Button>,
			);
		}

		right.push(
			...[
				<Button
					variant={"outline"}
					size={"icon"}
					onClick={async () => {
						toggleVars();
					}}
				>
					<VariableIcon />
				</Button>,
				<Button
					variant={"outline"}
					size={"icon"}
					onClick={async () => {
						setEditBoard(true);
					}}
				>
					<NotebookPenIcon />
				</Button>,
				<Button
					variant={"outline"}
					size={"icon"}
					onClick={async () => {
						toggleRunHistory();
					}}
				>
					<HistoryIcon />
				</Button>,
			],
		);

		// Always expose Logs button; it opens the logs sheet (shows empty state when no run is selected)
		right.push(
			<Button
				variant={"outline"}
				size={"icon"}
				aria-label="Open logs"
				onClick={async () => {
					toggleLogs();
				}}
			>
				<ScrollIcon />
			</Button>,
		);

		if (currentLayer) {
			left.push(
				<Button
					variant={"default"}
					size={"icon"}
					onClick={async () => {
						popLayer();
					}}
				>
					<SquareChevronUpIcon />
				</Button>,
			);
		}

		updateHeader({
			left,
			right,
		});
	}, [currentMetadata, currentLayer, parentRegister.boardParents, boardId]);

	const pinToNode = useCallback(
		(pinId: string) => {
			const [_, node] = pinCache.get(pinId) || [];
			return node;
		},
		[nodes, pinCache],
	);

	const { saveViewport } = useViewportManager({
		appId,
		boardId,
		layerPath,
		nodesLength: nodes.length,
	});

	const { focusNode, pushLayer, popLayer } = useLayerNavigation({
		board,
		layerPath,
		setCurrentLayer,
		setLayerPath,
		saveViewport,
		fitView,
	});

	const {
		executeCommand,
		executeCommands,
		awarenessRef: commandAwarenessRef,
	} = useCommandExecution({
		appId,
		boardId,
		board,
		version,
		pushCommand,
		pushCommands,
	});

	// Realtime collaboration
	const { awareness, connectionStatus, peerStates, reconnect } =
		useRealtimeCollaboration({
			appId,
			boardId,
			board,
			version,
			backend,
			currentProfile,
			hub,
			mousePosition,
			layerPath,
			screenToFlowPosition,
			commandAwarenessRef,
			setNodes,
		});

	useEffect(() => {
		if (!logPanelRef.current) return;
		// Avoid auto-expanding logs on mobile to prevent layout jump
		const isMobile =
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches;
		if (isMobile) return;
		logPanelRef.current.expand();
		const size = logPanelRef.current.getSize();
		if (size < 10) logPanelRef.current.resize(45);
	}, [logPanelRef.current]);

	const initializeFlow = useCallback(
		async (instance: ReactFlowInstance) => {
			if (initialized) return;
			if (!nodeId || nodeId === "") return;

			instance.fitView({
				nodes: [
					{
						id: nodeId ?? "",
					},
				],
				duration: 500,
			});
			setInitialized(true);
		},
		[nodeId, initialized],
	);

	const [varsOpen, setVarsOpen] = useState(false);
	const [runsOpen, setRunsOpen] = useState(false);
	const [logsOpen, setLogsOpen] = useState(false);

	const { toggleVars, toggleRunHistory, toggleLogs } = useFlowPanels({
		varPanelRef,
		runsPanelRef,
		logPanelRef,
		setVarsOpen,
		setRunsOpen,
		setLogsOpen,
	});

	// Clear selections when version changes
	useEffect(() => {
		selected.current.clear();
		setNodes((nds) =>
			nds.map((node) => ({
				...node,
				selected: false,
			})),
		);
		setEdges((eds) =>
			eds.map((edge) => ({
				...edge,
				selected: false,
			})),
		);
	}, [version, setNodes, setEdges]);

	const onMoveEnd = useCallback(() => {
		void saveViewport();
	}, [saveViewport]);

	const executeBoard = useCallback(
		async (node: INode, payload?: object) => {
			let added = false;
			let runId = "";
			let meta: ILogMetadata | undefined = undefined;
			try {
				meta = await backend.boardState.executeBoard(
					appId,
					boardId,
					{
						id: node.id,
						payload: payload,
					},
					true,
					async (id: string) => {
						if (added) return;
						console.log("Run started", id);
						runId = id;
						added = true;
						addRun(id, boardId, [node.id]);
					},
					(update) => {
						const runUpdates = update
							.filter((item) => item.event_type.startsWith("run:"))
							.map((item) => item.payload);
						if (runUpdates.length === 0) return;
						const firstItem = runUpdates[0];
						if (!added) {
							runId = firstItem.run_id;
							addRun(firstItem.run_id, boardId, [node.id]);
							added = true;
						}

						pushUpdate(firstItem.run_id, runUpdates);
					},
				);
			} catch (error) {
				console.warn("Failed to execute board", error);
			}
			removeRun(runId);
			if (!meta) {
				toastError(
					"Failed to execute board",
					<PlayCircleIcon className="w-4 h-4" />,
				);
				return;
			}
			await refetchLogs(backend);
			if (meta) setCurrentMetadata(meta);
		},
		[
			appId,
			boardId,
			backend,
			refetchLogs,
			pushUpdate,
			addRun,
			removeRun,
			setCurrentMetadata,
		],
	);

	const handlePasteCB = useCallback(
		async (event: ClipboardEvent) => {
			if (typeof version !== "undefined") {
				toastError("Cannot change old version", <XIcon />);
				return;
			}
			const currentCursorPosition = screenToFlowPosition({
				x: mousePosition.x,
				y: mousePosition.y,
			});
			await handlePaste(
				event,
				currentCursorPosition,
				boardId,
				executeCommand,
				currentLayer,
			);
		},
		[boardId, mousePosition, executeCommand, currentLayer, version],
	);

	const handleCopyCB = useCallback(
		(event?: ClipboardEvent) => {
			if (!board.data) return;
			const currentCursorPosition = screenToFlowPosition({
				x: mousePosition.x,
				y: mousePosition.y,
			});
			handleCopy(nodes, board.data, currentCursorPosition, event, currentLayer);
		},
		[nodes, mousePosition, board.data, currentLayer],
	);

	const openNodeInfo = useCallback((node: INode) => {
		nodeInfoOverlayRef.current?.openNodeInfo(node);
	}, []);

	const placeNodeShortcut = useCallback(
		async (node: INode) => {
			await placeNode(node, {
				x: mousePosition.x,
				y: mousePosition.y,
			});
		},
		[mousePosition],
	);

	const placeNode = useCallback(
		async (node: INode, position?: { x: number; y: number }) => {
			const location = screenToFlowPosition({
				x: position?.x ?? clickPosition.x,
				y: position?.y ?? clickPosition.y,
			});

			await handlePlaceNode({
				node,
				position: location,
				droppedPin,
				currentLayer,
				refs: board.data?.refs ?? {},
				boardNodes: board.data?.nodes ?? {},
				pinCache,
				executeCommand,
			});
		},
		[
			clickPosition,
			droppedPin,
			board.data?.refs,
			board.data?.nodes,
			currentLayer,
			screenToFlowPosition,
			pinCache,
			executeCommand,
		],
	);

	const placePlaceholder = useCallback(
		async (name: string, position?: { x: number; y: number }) => {
			const delayNode = catalog.data?.find((node) => node.name === "delay");
			const location = screenToFlowPosition({
				x: position?.x ?? clickPosition.x,
				y: position?.y ?? clickPosition.y,
			});

			await handlePlacePlaceholder({
				name,
				position: location,
				droppedPin,
				currentLayer,
				refs: board.data?.refs ?? {},
				pinCache,
				delayNode,
				executeCommand,
				executeCommands,
			});
		},
		[
			clickPosition,
			droppedPin,
			board.data?.refs,
			executeCommand,
			executeCommands,
			pinCache,
			currentLayer,
			screenToFlowPosition,
			catalog.data,
		],
	);

	useKeyboardShortcuts({
		board,
		catalog,
		version,
		appId,
		boardId,
		mousePosition,
		placeNode,
		undo,
		redo,
	});

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
		boardRef.current = board.data;
		const parsed = parseBoard(
			board.data,
			appId,
			handleCopyCB,
			pushLayer,
			executeBoard,
			executeCommand,
			selected.current,
			currentProfile.data?.settings?.connection_mode ?? "default",
			nodes,
			edges,
			currentLayer,
			boardRef,
			version,
			openNodeInfo,
		);

		setNodes(parsed.nodes);
		setEdges(parsed.edges);
		setPinCache(new Map(parsed.cache));
	}, [board.data, currentLayer, currentProfile.data, version, openNodeInfo]);

	const nodeTypes = useMemo(
		() => ({
			flowNode: FlowNode,
			commentNode: CommentNode,
			layerNode: LayerNode,
			layerInnerNode: LayerInnerNode,
			node: FlowNode,
		}),
		[],
	);

	const edgeTypes = useMemo(
		() => ({
			veil: FlowVeilEdge,
			execution: FlowExecutionEdge,
			data: FlowDataEdge,
		}),
		[],
	);

	const onConnect = useCallback(
		(params: any) =>
			setEdges((eds) =>
				handleConnection({
					params,
					version,
					boardNodes: board.data?.nodes ?? {},
					pinCache,
					executeCommand,
					addEdge: (p: any, e: any[]) => addEdge(p, e),
					currentEdges: eds,
				}),
			),
		[setEdges, pinCache, version, executeCommand, board.data?.nodes],
	);

	const onSelectionChange = useCallback<OnSelectionChangeFunc<Node, Edge>>(
		({ nodes: selectedNodes }) => {
			if (!awareness) return;
			const nodeIds = selectedNodes
				.filter((selectedNode) => selectedNode.type === "node")
				.map((selectedNode) => selectedNode.id);
			awareness.setLocalStateField("selection", { nodes: nodeIds });
		},
		[awareness],
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
					// Check if this is a function reference handle
					if (
						handle.id.startsWith("ref_in_") ||
						handle.id.startsWith("ref_out_")
					) {
						// Create a synthetic pin object for ref handles
						const syntheticPin: IPin = {
							id: handle.id,
							name: handle.id.startsWith("ref_in_") ? "ref_in" : "ref_out",
							friendly_name: handle.id.startsWith("ref_in_")
								? "Function Reference In"
								: "Function Reference Out",
							pin_type: handle.id.startsWith("ref_in_")
								? IPinType.Input
								: IPinType.Output,
							data_type: IVariableType.Generic,
							value_type: IValueType.Normal,
							depends_on: [],
							connected_to: [],
							index: 0,
							description: "",
							schema: "",
						};
						setDroppedPin(syntheticPin);
					} else {
						const [pin, _node] = pinCache.get(handle.id) || [];
						setDroppedPin(pin);
					}
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
			setNodes((nds) =>
				handleNodesChange({
					changes,
					currentNodes: nds,
					selected,
					version,
					boardData: board.data,
					deletingNodesRef,
					executeCommands,
					applyNodeChanges,
				}),
			),
		[setNodes, board.data, executeCommands, version],
	);

	const onEdgesChange: OnEdgesChange = useCallback(
		(changes: any[]) =>
			setEdges((eds) =>
				handleEdgesChange({
					changes,
					currentEdges: eds,
					selected,
					version,
					boardData: board.data,
					pinCache,
					deletingNodesRef,
					executeCommands,
					applyEdgeChanges,
				}),
			),
		[setEdges, board.data, pinCache, executeCommands, version],
	);

	const onReconnectStart = useCallback(() => {
		edgeReconnectSuccessful.current = false;
	}, []);

	const onReconnect = useCallback(
		async (oldEdge: any, newConnection: Connection) => {
			// Don't execute commands when viewing an old version
			if (typeof version !== "undefined") {
				return;
			}

			// Check if the edge is actually being moved
			const new_id = `${newConnection.sourceHandle}-${newConnection.targetHandle}`;
			if (oldEdge.id === new_id) {
				return;
			}

			// Check if this is a veil edge (fn_ref) FIRST - handle it differently
			const isOldRefConnection =
				(oldEdge.sourceHandle?.startsWith("ref_out_") &&
					oldEdge.targetHandle?.startsWith("ref_in_")) ||
				(oldEdge.sourceHandle?.startsWith("ref_in_") &&
					oldEdge.targetHandle?.startsWith("ref_out_"));
			const isNewRefConnection =
				(newConnection.sourceHandle?.startsWith("ref_out_") &&
					newConnection.targetHandle?.startsWith("ref_in_")) ||
				(newConnection.sourceHandle?.startsWith("ref_in_") &&
					newConnection.targetHandle?.startsWith("ref_out_"));

			if (isOldRefConnection && isNewRefConnection) {
				const oldSource = oldEdge.sourceHandle;
				const oldTarget = oldEdge.targetHandle;
				const newSource = newConnection.sourceHandle;
				const newTarget = newConnection.targetHandle;

				// Determine which end was reconnected
				const sourceChanged = oldSource !== newSource;
				const targetChanged = oldTarget !== newTarget;

				const commands: IGenericCommand[] = [];

				if (sourceChanged) {
					// Source (ref_out) was reconnected - update both old and new source nodes
					const oldRefOutNodeId = oldSource?.replace("ref_out_", "") || "";
					const newRefOutNodeId = newSource?.replace("ref_out_", "") || "";
					const refInNodeId = oldTarget?.replace("ref_in_", "") || "";

					const oldRefOutNode = board.data?.nodes[oldRefOutNodeId];
					const newRefOutNode = board.data?.nodes[newRefOutNodeId];

					// Remove ref from old source node
					if (oldRefOutNode && refInNodeId) {
						const currentRefs = oldRefOutNode.fn_refs?.fn_refs ?? [];
						const updatedRefs = currentRefs.filter(
							(ref: string) => ref !== refInNodeId,
						);

						const updatedOldNode = {
							...oldRefOutNode,
							fn_refs: {
								...oldRefOutNode.fn_refs,
								fn_refs: updatedRefs,
								can_reference_fns:
									oldRefOutNode.fn_refs?.can_reference_fns ?? false,
								can_be_referenced_by_fns:
									oldRefOutNode.fn_refs?.can_be_referenced_by_fns ?? false,
							},
						};

						commands.push(updateNodeCommand({ node: updatedOldNode }));
					}

					// Add ref to new source node
					if (newRefOutNode && refInNodeId) {
						const currentRefs = newRefOutNode.fn_refs?.fn_refs ?? [];
						const updatedRefs = [...currentRefs];

						if (!updatedRefs.includes(refInNodeId)) {
							updatedRefs.push(refInNodeId);
						}

						const updatedNewNode = {
							...newRefOutNode,
							fn_refs: {
								...newRefOutNode.fn_refs,
								fn_refs: updatedRefs,
								can_reference_fns:
									newRefOutNode.fn_refs?.can_reference_fns ?? false,
								can_be_referenced_by_fns:
									newRefOutNode.fn_refs?.can_be_referenced_by_fns ?? false,
							},
						};

						commands.push(updateNodeCommand({ node: updatedNewNode }));
					}
				} else if (targetChanged) {
					// Target (ref_in) was reconnected - update the source node's refs
					const refOutNodeId = oldSource?.replace("ref_out_", "") || "";
					const oldRefInNodeId = oldTarget?.replace("ref_in_", "") || "";
					const newRefInNodeId = newTarget?.replace("ref_in_", "") || "";

					const refOutNode = board.data?.nodes[refOutNodeId];

					if (refOutNode && newRefInNodeId && oldRefInNodeId) {
						const currentRefs = refOutNode.fn_refs?.fn_refs ?? [];

						// Remove old ref, add new ref
						const updatedRefs = currentRefs.filter(
							(ref: string) => ref !== oldRefInNodeId,
						);

						if (!updatedRefs.includes(newRefInNodeId)) {
							updatedRefs.push(newRefInNodeId);
						}

						const updatedNode = {
							...refOutNode,
							fn_refs: {
								...refOutNode.fn_refs,
								fn_refs: updatedRefs,
								can_reference_fns:
									refOutNode.fn_refs?.can_reference_fns ?? false,
								can_be_referenced_by_fns:
									refOutNode.fn_refs?.can_be_referenced_by_fns ?? false,
							},
						};

						commands.push(updateNodeCommand({ node: updatedNode }));
					}
				}

				if (commands.length > 0) {
					await executeCommands(commands);
				}
			} else {
				// Regular pin connection reconnection - need to look up nodes
				const oldEdgeToNode = pinToNode(oldEdge.targetHandle);
				const oldEdgeFromNode = pinToNode(oldEdge.sourceHandle);

				if (!oldEdgeToNode || !oldEdgeFromNode) {
					return;
				}

				const commands = [];

				const disconnectCommand = disconnectPinsCommand({
					from_node: oldEdgeFromNode.id,
					from_pin: oldEdge.sourceHandle,
					to_node: oldEdgeToNode.id,
					to_pin: oldEdge.targetHandle,
				});

				commands.push(disconnectCommand);

				if (newConnection.targetHandle && newConnection.sourceHandle) {
					const newConnectionSourceNode = pinToNode(newConnection.sourceHandle);
					const newConnectionTargetNode = pinToNode(newConnection.targetHandle);

					if (newConnectionSourceNode && newConnectionTargetNode)
						commands.push(
							connectPinsCommand({
								from_node: newConnectionSourceNode.id,
								from_pin: newConnection.sourceHandle,
								to_node: newConnectionTargetNode.id,
								to_pin: newConnection.targetHandle,
							}),
						);
				}

				await executeCommands(commands);
			}

			edgeReconnectSuccessful.current = true;
			setEdges((els) => reconnectEdge(oldEdge, newConnection, els));
		},
		[
			setEdges,
			pinToNode,
			executeCommands,
			executeCommand,
			board.data?.nodes,
			version,
		],
	);

	const onReconnectEnd = useCallback(
		async (event: any, edge: any) => {
			// Don't execute commands when viewing an old version
			if (typeof version !== "undefined") {
				return;
			}

			if (!edgeReconnectSuccessful.current) {
				const { source, target, sourceHandle, targetHandle } = edge;
				const from_node = pinToNode(sourceHandle);
				const to_node = pinToNode(targetHandle);
				if (!from_node || !to_node) return;
				const command = disconnectPinsCommand({
					from_node: from_node?.id,
					from_pin: sourceHandle,
					to_node: to_node?.id,
					to_pin: targetHandle,
				});
				await executeCommand(command);
				setEdges((eds) => eds.filter((e) => e.id !== edge.id));
			}

			edgeReconnectSuccessful.current = true;
		},
		[setEdges, pinToNode, version, executeCommand],
	);

	const onContextMenuCB = useCallback((event: any) => {
		setClickPosition({ x: event.clientX, y: event.clientY });
	}, []);

	const onNodeDragStop = useCallback(
		async (event: any, node: any, nodes: any) => {
			// Don't execute commands when viewing an old version
			if (typeof version !== "undefined") {
				return;
			}
			const commands: IGenericCommand[] = [];
			for await (const node of nodes) {
				const command = moveNodeCommand({
					node_id: node.id,
					to_coordinates: [node.position.x, node.position.y, 0],
					current_layer: currentLayer,
				});

				commands.push(command);
			}
			await executeCommands(commands);
		},
		[boardId, executeCommands, currentLayer, version],
	);

	const isValidConnectionCB = useCallback(
		(connection: Edge | Connection) => {
			return isValidConnection(connection, pinCache, board.data?.refs ?? {});
		},
		[pinCache, board.data?.refs],
	) as IsValidConnection<Edge>;

	const onNodeDoubleClick = useCallback(
		(event: any, node: any) => {
			const tgt = event.target as HTMLElement;
			if (tgt.closest("input, textarea")) {
				return;
			}
			const type = node?.type ?? "";
			if (type === "layerNode") {
				const layer: ILayer = node.data.layer;
				pushLayer(layer);
				return;
			}
		},
		[pushLayer],
	);

	const onCommentPlace = useCallback(async () => {
		// Don't execute commands when viewing an old version
		if (typeof version !== "undefined") {
			return;
		}

		const location = screenToFlowPosition({
			x: clickPosition.x,
			y: clickPosition.y,
		});
		const new_comment: IComment = {
			comment_type: ICommentType.Text,
			content: "",
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
			current_layer: currentLayer,
		});

		await executeCommand(command);
	}, [currentLayer, clickPosition, executeCommand, version]);

	const onNodeDrag = useCallback(
		(event: any, node: Node, nodes: Node[]) => {
			if (shiftPressed) {
				nodes.forEach((node) => {
					if (node.type === "layerNode") {
						const layerData = node.data.layer as ILayer;
						const diffX = Math.abs(node.position.x - layerData.coordinates[0]);
						const diffY = Math.abs(node.position.y - layerData.coordinates[1]);
						if (diffX > diffY) {
							node.position.y = layerData.coordinates[1];
							return;
						}
						node.position.x = layerData.coordinates[0];
						return;
					}

					if (node.type === "commentNode") {
						const commentData = node.data.comment as IComment;
						const diffX = Math.abs(
							node.position.x - commentData.coordinates[0],
						);
						const diffY = Math.abs(
							node.position.y - commentData.coordinates[1],
						);
						if (diffX > diffY) {
							node.position.y = commentData.coordinates[1];
							return;
						}
						node.position.x = commentData.coordinates[0];
						return;
					}

					if (node.type === "node") {
						const nodeData = node.data.node as INode;
						if (!nodeData.coordinates) return;
						const diffX = Math.abs(node.position.x - nodeData.coordinates[0]);
						const diffY = Math.abs(node.position.y - nodeData.coordinates[1]);
						if (diffX > diffY) {
							node.position.y = nodeData.coordinates[1];
							return;
						}
						node.position.x = nodeData.coordinates[0];
					}
				});
			}
		},
		[shiftPressed],
	);

	const onAcceptSuggestion = useCallback(
		async (suggestion: any) => {
			const node = catalog.data?.find((n) => n.name === suggestion.node_type);
			if (node) {
				await placeNode(node);
			} else {
				toastError(`Node type ${suggestion.node_type} not found`, <XIcon />);
			}
		},
		[catalog.data, placeNode],
	);

	const [ghostNodes, setGhostNodes] = useState<
		{
			id: string;
			node_type: string;
			position: { x: number; y: number };
			reason: string;
		}[]
	>([]);

	const handleGhostNodesChange = useCallback((suggestions: Suggestion[]) => {
		setGhostNodes(
			suggestions.map((s, i) => ({
				id: `ghost-${i}`,
				node_type: s.node_type,
				position: s.position || { x: 0, y: 0 },
				reason: s.reason,
			})),
		);
	}, []);

	const handleExecuteCommands = useCallback(
		async (commands: BoardCommand[]) => {
			const boardNodes = board.data?.nodes ?? {};

			// ===== MAPPING TABLES =====
			// Maps node references (node_type, friendly_name, etc.) to actual created nodes
			const nodeReferenceMap = new Map<string, INode>();
			// Maps original pin names to actual pin IDs for each node (by node reference)
			// Structure: nodeRef -> (pinName -> actualPinId)
			const pinIdMap = new Map<string, Map<string, string>>();

			// ===== POSITION CALCULATION =====
			const existingNodes = Object.values(boardNodes);
			let baseX = 100;
			let baseY = 100;

			if (existingNodes.length > 0) {
				const rightmostNode = existingNodes.reduce((max, node) => {
					const x = node.coordinates?.[0] ?? 0;
					return x > (max.coordinates?.[0] ?? 0) ? node : max;
				});
				baseX = (rightmostNode.coordinates?.[0] ?? 0) + 300;
				baseY = rightmostNode.coordinates?.[1] ?? 100;
			}

			// ===== HELPER FUNCTIONS =====

			// Resolve a node reference to actual node (from board or newly created)
			const resolveNode = (ref: string): INode | undefined => {
				// Check existing board nodes first
				if (boardNodes[ref]) return boardNodes[ref];
				// Check newly created nodes by reference
				return nodeReferenceMap.get(ref);
			};

			// Resolve a pin reference to actual pin ID
			// Uses the pin mapping table first, then falls back to searching by name
			const resolvePinId = (
				nodeRef: string,
				pinRef: string,
			): string | undefined => {
				// Check pin mapping table - try exact match first, then lowercase
				const nodePinMap = pinIdMap.get(nodeRef);
				if (nodePinMap) {
					if (nodePinMap.has(pinRef)) {
						return nodePinMap.get(pinRef);
					}
					// Try lowercase lookup
					if (nodePinMap.has(pinRef.toLowerCase())) {
						return nodePinMap.get(pinRef.toLowerCase());
					}
				}

				// Fallback: resolve node and search by pin name
				const node = resolveNode(nodeRef);
				if (!node) return undefined;

				// Try exact ID match
				if (node.pins[pinRef]) return pinRef;

				// Search by name (case-insensitive)
				for (const pin of Object.values(node.pins)) {
					if (pin.name.toLowerCase() === pinRef.toLowerCase()) return pin.id;
					if (pin.friendly_name?.toLowerCase() === pinRef.toLowerCase())
						return pin.id;
				}

				// Log available pins for debugging
				console.warn(
					`Pin "${pinRef}" not found in node "${node.friendly_name || node.name}". Available pins:`,
					Object.values(node.pins).map((p) => ({
						id: p.id,
						name: p.name,
						type: p.pin_type,
					})),
				);
				return undefined;
			};

			// Build pin mapping for a node (maps pin names to actual IDs)
			const buildPinMapping = (nodeRef: string, node: INode) => {
				const pinMap = new Map<string, string>();
				for (const pin of Object.values(node.pins)) {
					// Map by pin name (primary key for lookups)
					pinMap.set(pin.name, pin.id);
					pinMap.set(pin.name.toLowerCase(), pin.id);
					// Also map by friendly_name if different
					if (pin.friendly_name && pin.friendly_name !== pin.name) {
						pinMap.set(pin.friendly_name, pin.id);
						pinMap.set(pin.friendly_name.toLowerCase(), pin.id);
					}
				}
				pinIdMap.set(nodeRef, pinMap);
				return pinMap;
			};

			// ===== BUILD PIN MAPPINGS FOR EXISTING BOARD NODES =====
			// Pre-build pin mappings for all existing nodes so connections work reliably
			for (const [nodeId, node] of Object.entries(boardNodes)) {
				buildPinMapping(nodeId, node);
			}

			// ===== FIRST PASS: ADD NODES =====
			// Execute all AddNode commands first to build the mapping tables
			let nodeIndex = 0;

			for (const cmd of commands) {
				if (cmd.command_type === "AddNode") {
					const catalogNode = catalog.data?.find(
						(n) => n.name === cmd.node_type,
					);
					if (!catalogNode) {
						toastError(
							`Node type ${cmd.node_type} not found in catalog`,
							<XIcon />,
						);
						continue;
					}

					const position = cmd.position || {
						x: baseX + (nodeIndex % 3) * 300,
						y: baseY + Math.floor(nodeIndex / 3) * 200,
					};

					// ALWAYS use catalog's friendly_name - ignore any custom name from copilot
					const result = addNodeCommand({
						node: {
							...catalogNode,
							coordinates: [position.x, position.y, 0],
							friendly_name: catalogNode.friendly_name, // Always use catalog name
						},
						current_layer: currentLayer,
					});

					// Execute the command - the returned command has the node with final IDs
					const executedCommand = await executeCommand(result.command);

					// Check if command execution failed (returns undefined if version is set or backend is missing)
					if (!executedCommand) {
						console.error(
							`[AddNode] Command execution returned undefined - command may not have been executed`,
						);
						toastError(
							`Failed to add node "${catalogNode.friendly_name}" - check if you're editing the latest version`,
							<XIcon />,
						);
						continue;
					}

					// CRITICAL: Use the node from the executed command, which has the IDs that are actually in the board
					// The executedCommand.node is the same object we sent (frontend generates IDs)
					// result.node is a reference to the same object
					const actualNode = (executedCommand?.node as INode) ?? result.node;

					// Store node references for lookup using multiple keys
					// Include ref_id (e.g., "$0", "$1") which the LLM uses to reference new nodes
					const refs = [
						cmd.ref_id, // Primary reference: "$0", "$1", "$2", etc.
						`$${nodeIndex}`, // Also accept index format
						cmd.node_type,
						actualNode.id,
					].filter(Boolean) as string[];

					for (const ref of refs) {
						nodeReferenceMap.set(ref, actualNode);
						buildPinMapping(ref, actualNode);
					}

					console.log(
						`[AddNode] Created "${actualNode.friendly_name}" (${actualNode.id})`,
						{
							refs,
							pins: Object.values(actualNode.pins).map(
								(p) => `${p.name}:${p.id.slice(0, 8)}`,
							),
						},
					);

					nodeIndex++;
				}
			}

			// Small delay helper to ensure UI updates between commands
			const delay = (ms: number) =>
				new Promise((resolve) => setTimeout(resolve, ms));

			// ===== SECOND PASS: CONNECTIONS & UPDATES =====
			// Now execute connections and updates using the mapping tables
			for (const cmd of commands) {
				// Small delay between commands for UI responsiveness
				await delay(50);
				switch (cmd.command_type) {
					case "AddNode":
						// Already handled
						break;

					case "RemoveNode": {
						const node = resolveNode(cmd.node_id);
						if (node) {
							await executeCommand(
								removeNodeCommand({
									node,
									connected_nodes: [],
								}),
							);
						}
						break;
					}

					case "ConnectPins": {
						const fromNode = resolveNode(cmd.from_node);
						const toNode = resolveNode(cmd.to_node);

						if (!fromNode || !toNode) {
							const missingNode = !fromNode ? cmd.from_node : cmd.to_node;
							console.error(
								`[ConnectPins] ❌ FAILED - Node not found: "${missingNode}"`,
								{
									command: cmd,
									availableNodeRefs: Array.from(nodeReferenceMap.keys()),
									boardNodeIds: Object.keys(boardNodes),
								},
							);
							toastError(
								`Connection failed: Node "${missingNode}" not found`,
								<XIcon />,
							);
							break;
						}

						const fromPinId = resolvePinId(cmd.from_node, cmd.from_pin);
						const toPinId = resolvePinId(cmd.to_node, cmd.to_pin);

						if (!fromPinId || !toPinId) {
							const missingPin = !fromPinId
								? `${fromNode.friendly_name}.${cmd.from_pin}`
								: `${toNode.friendly_name}.${cmd.to_pin}`;
							console.error(
								`[ConnectPins] ❌ FAILED - Pin not found: "${missingPin}"`,
								{
									command: cmd,
									from_pin_requested: cmd.from_pin,
									to_pin_requested: cmd.to_pin,
									fromPinId_resolved: fromPinId,
									toPinId_resolved: toPinId,
									fromNodePins: Object.values(fromNode.pins).map((p) => ({
										name: p.name,
										id: p.id,
										type: p.pin_type,
									})),
									toNodePins: Object.values(toNode.pins).map((p) => ({
										name: p.name,
										id: p.id,
										type: p.pin_type,
									})),
								},
							);
							toastError(
								`Connection failed: Pin "${missingPin}" not found`,
								<XIcon />,
							);
							break;
						}

						console.log(
							`[ConnectPins] ✓ Connecting: ${fromNode.friendly_name}.${cmd.from_pin} -> ${toNode.friendly_name}.${cmd.to_pin}`,
							{
								from_node_id: fromNode.id,
								from_pin_id: fromPinId,
								to_node_id: toNode.id,
								to_pin_id: toPinId,
							},
						);

						try {
							const connectResult = await executeCommand(
								connectPinsCommand({
									from_node: fromNode.id,
									from_pin: fromPinId,
									to_node: toNode.id,
									to_pin: toPinId,
								}),
							);
							if (connectResult) {
								console.log(`[ConnectPins] ✓ SUCCESS:`, connectResult);
								// Wait for backend to process the connection before continuing
								// This ensures subsequent UpdateNodePin commands see the connected state
								await delay(100);
							} else {
								console.error(
									`[ConnectPins] ❌ FAILED - executeCommand returned undefined/null`,
								);
								toastError(
									`Connection failed: Command not executed (check version)`,
									<XIcon />,
								);
							}
						} catch (err) {
							console.error(`[ConnectPins] ❌ FAILED - Exception:`, err);
							toastError(`Connection failed: ${err}`, <XIcon />);
						}
						break;
					}

					case "DisconnectPins": {
						const fromNode = resolveNode(cmd.from_node);
						const toNode = resolveNode(cmd.to_node);
						if (!fromNode || !toNode) break;

						const fromPinId = resolvePinId(cmd.from_node, cmd.from_pin);
						const toPinId = resolvePinId(cmd.to_node, cmd.to_pin);
						if (!fromPinId || !toPinId) break;

						await executeCommand(
							disconnectPinsCommand({
								from_node: fromNode.id,
								from_pin: fromPinId,
								to_node: toNode.id,
								to_pin: toPinId,
							}),
						);
						break;
					}

					case "UpdateNodePin": {
						// CRITICAL: Refetch board to get latest node state (connections may have changed)
						const freshBoard = await board.refetch();
						const freshNodes = freshBoard.data?.nodes ?? {};

						// Resolve node from fresh data first, then fall back to reference map
						const nodeId = nodeReferenceMap.get(cmd.node_id)?.id ?? cmd.node_id;
						const node = freshNodes[nodeId] ?? resolveNode(cmd.node_id);

						if (!node) {
							console.error(
								`[UpdateNodePin] ❌ FAILED - Node not found: ${cmd.node_id}`,
								{
									command: cmd,
									availableNodeRefs: Array.from(nodeReferenceMap.keys()),
									boardNodeIds: Object.keys(freshNodes),
								},
							);
							toastError(
								`Pin update failed: Node "${cmd.node_id}" not found`,
								<XIcon />,
							);
							break;
						}

						const pinId = resolvePinId(cmd.node_id, cmd.pin_id);
						const pin = pinId ? node.pins[pinId] : undefined;

						if (!pin || !pinId) {
							console.error(
								`[UpdateNodePin] ❌ FAILED - Pin not found: ${cmd.pin_id} in ${node.friendly_name}`,
								{
									command: cmd,
									pin_requested: cmd.pin_id,
									pinId_resolved: pinId,
									availablePins: Object.values(node.pins).map((p) => ({
										name: p.name,
										id: p.id,
										type: p.pin_type,
									})),
								},
							);
							toastError(
								`Pin update failed: Pin "${cmd.pin_id}" not found in "${node.friendly_name}"`,
								<XIcon />,
							);
							break;
						}

						let encodedValue: number[] | null = null;
						if (cmd.value !== null && cmd.value !== undefined) {
							// For strings, encode the raw string directly without JSON.stringify
							// to avoid double-quoting (e.g., "felix" becoming "\"felix\"")
							let valueToEncode = cmd.value;
							if (typeof cmd.value === "string") {
								// Remove surrounding quotes if present (from JSON parsing)
								if (cmd.value.startsWith('"') && cmd.value.endsWith('"')) {
									valueToEncode = cmd.value.slice(1, -1);
								}
							}
							const encoded = convertJsonToUint8Array(valueToEncode);
							if (encoded) {
								encodedValue = encoded;
							} else {
								console.error(
									`[UpdateNodePin] ❌ FAILED - Could not encode value:`,
									cmd.value,
								);
								toastError(
									`Pin update failed: Could not encode value`,
									<XIcon />,
								);
								break;
							}
						}
						console.log(
							`[UpdateNodePin] ✓ Setting: ${node.friendly_name}.${cmd.pin_id} = ${JSON.stringify(cmd.value)}`,
							{ encodedValue, originalValue: cmd.value, pinId },
						);

						// Create updated node with the modified pin
						const updatedNode: INode = {
							...node,
							pins: {
								...node.pins,
								[pinId]: {
									...pin,
									default_value: encodedValue,
								},
							},
						};

						try {
							const result = await executeCommand(
								updateNodeCommand({
									node: updatedNode,
									old_node: node,
								}),
							);
							if (result) {
								console.log(`[UpdateNodePin] ✓ SUCCESS:`, result);
							} else {
								console.error(
									`[UpdateNodePin] ❌ FAILED - executeCommand returned undefined/null`,
								);
								toastError(
									`Pin update failed: Command not executed (check version)`,
									<XIcon />,
								);
							}
						} catch (err) {
							console.error(`[UpdateNodePin] ❌ FAILED - Exception:`, err);
							toastError(`Pin update failed: ${err}`, <XIcon />);
						}
						break;
					}

					case "MoveNode": {
						const node = resolveNode(cmd.node_id);
						if (!node) break;

						await executeCommand(
							moveNodeCommand({
								node_id: node.id,
								to_coordinates: [cmd.position.x, cmd.position.y, 0],
								current_layer: currentLayer,
							}),
						);
						break;
					}

					case "CreateVariable": {
						const variableId = createId();
						const variable: IVariable = {
							id: variableId,
							name: cmd.name,
							data_type:
								(cmd.data_type as IVariableType) || IVariableType.String,
							value_type: (cmd.value_type as IValueType) || IValueType.Normal,
							default_value: cmd.default_value
								? Array.from(convertJsonToUint8Array(cmd.default_value) || [])
								: null,
							description: cmd.description || null,
							editable: true,
							exposed: false,
							secret: false,
						};

						console.log(`[CreateVariable] ${cmd.name} (${cmd.data_type})`);
						await executeCommand(upsertVariableCommand({ variable }));
						break;
					}

					case "UpdateVariable": {
						const existingVariable = board.data?.variables?.[cmd.variable_id];
						if (!existingVariable) {
							toastError(
								`Cannot update variable: "${cmd.variable_id}" not found`,
								<XIcon />,
							);
							break;
						}

						const updatedVariable: IVariable = {
							...existingVariable,
							default_value: cmd.value
								? Array.from(convertJsonToUint8Array(cmd.value) || [])
								: existingVariable.default_value,
						};

						console.log(
							`[UpdateVariable] ${existingVariable.name} = ${JSON.stringify(cmd.value)}`,
						);
						await executeCommand(
							upsertVariableCommand({
								variable: updatedVariable,
								old_variable: existingVariable,
							}),
						);
						break;
					}

					case "DeleteVariable": {
						const variableToDelete = board.data?.variables?.[cmd.variable_id];
						if (!variableToDelete) {
							toastError(
								`Cannot delete variable: "${cmd.variable_id}" not found`,
								<XIcon />,
							);
							break;
						}

						console.log(`[DeleteVariable] ${variableToDelete.name}`);
						await executeCommand(
							removeVariableCommand({ variable: variableToDelete }),
						);
						break;
					}

					case "CreateComment": {
						const commentId = createId();
						const comment: IComment = {
							id: commentId,
							content: cmd.content,
							comment_type: ICommentType.Text,
							coordinates: cmd.position
								? [cmd.position.x, cmd.position.y, 0]
								: [baseX, baseY, 0],
							color: cmd.color || null,
							timestamp: {
								nanos_since_epoch: 0,
								secs_since_epoch: Math.floor(Date.now() / 1000),
							},
							author: "copilot",
						};

						console.log(`[CreateComment] "${cmd.content.slice(0, 30)}..."`);
						await executeCommand(
							upsertCommentCommand({ comment, current_layer: currentLayer }),
						);
						break;
					}

					case "UpdateComment": {
						const existingComment = board.data?.comments?.[cmd.comment_id];
						if (!existingComment) {
							toastError(
								`Cannot update comment: "${cmd.comment_id}" not found`,
								<XIcon />,
							);
							break;
						}

						const updatedComment: IComment = {
							...existingComment,
							content: cmd.content ?? existingComment.content,
							color: cmd.color ?? existingComment.color,
						};

						console.log(
							`[UpdateComment] "${updatedComment.content.slice(0, 30)}..."`,
						);
						await executeCommand(
							upsertCommentCommand({
								comment: updatedComment,
								current_layer: currentLayer,
								old_comment: existingComment,
							}),
						);
						break;
					}

					case "DeleteComment": {
						const commentToDelete = board.data?.comments?.[cmd.comment_id];
						if (!commentToDelete) {
							toastError(
								`Cannot delete comment: "${cmd.comment_id}" not found`,
								<XIcon />,
							);
							break;
						}

						console.log(
							`[DeleteComment] "${commentToDelete.content.slice(0, 30)}..."`,
						);
						await executeCommand(
							removeCommentCommand({ comment: commentToDelete }),
						);
						break;
					}

					case "CreateLayer": {
						const layerId = createId();
						const layer: ILayer = {
							id: layerId,
							name: cmd.name,
							type: ILayerType.Collapsed,
							color: cmd.color || null,
							coordinates: [baseX, baseY, 0],
							nodes: {},
							variables: {},
							comments: {},
							pins: {},
							parent_id: currentLayer,
						};

						console.log(
							`[CreateLayer] "${cmd.name}" with ${cmd.node_ids?.length || 0} nodes`,
						);
						await executeCommand(
							upsertLayerCommand({
								layer,
								node_ids: cmd.node_ids || [],
								current_layer: currentLayer,
							}),
						);
						break;
					}

					case "AddNodesToLayer": {
						const existingLayer = board.data?.layers?.[cmd.layer_id];
						if (!existingLayer) {
							toastError(
								`Cannot add nodes to layer: "${cmd.layer_id}" not found`,
								<XIcon />,
							);
							break;
						}

						const existingNodeIds = Object.keys(existingLayer.nodes || {});
						const allNodeIds = [
							...new Set([...existingNodeIds, ...cmd.node_ids]),
						];

						console.log(
							`[AddNodesToLayer] Adding ${cmd.node_ids.length} nodes to "${existingLayer.name}"`,
						);
						await executeCommand(
							upsertLayerCommand({
								layer: existingLayer,
								node_ids: allNodeIds,
								current_layer: currentLayer,
								old_layer: existingLayer,
							}),
						);
						break;
					}

					case "RemoveNodesFromLayer": {
						const layerToUpdate = board.data?.layers?.[cmd.layer_id];
						if (!layerToUpdate) {
							toastError(
								`Cannot remove nodes from layer: "${cmd.layer_id}" not found`,
								<XIcon />,
							);
							break;
						}

						const currentNodeIds = Object.keys(layerToUpdate.nodes || {});
						const remainingNodeIds = currentNodeIds.filter(
							(id) => !cmd.node_ids.includes(id),
						);

						console.log(
							`[RemoveNodesFromLayer] Removing ${cmd.node_ids.length} nodes from "${layerToUpdate.name}"`,
						);
						await executeCommand(
							upsertLayerCommand({
								layer: layerToUpdate,
								node_ids: remainingNodeIds,
								current_layer: currentLayer,
								old_layer: layerToUpdate,
							}),
						);
						break;
					}
				}
			}

			console.log(
				`[handleExecuteCommands] Completed ${commands.length} commands`,
			);

			// Final refetch to ensure UI is in sync
			await board.refetch();
		},
		[
			catalog.data,
			executeCommand,
			board.data?.nodes,
			board.data?.variables,
			board.data?.comments,
			board.data?.layers,
			currentLayer,
			board.refetch,
		],
	);

	return (
		<div className="w-full flex-1 grow flex-col min-h-0 relative">
			<FlowCopilot
				board={board.data}
				selectedNodeIds={Array.from(selected.current)}
				onAcceptSuggestion={onAcceptSuggestion}
				onFocusNode={focusNode}
				onGhostNodesChange={handleGhostNodesChange}
				onExecuteCommands={handleExecuteCommands}
			/>
			{/* Realtime connection status indicator */}
			{awareness && connectionStatus === "connected" && (
				<div className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--primary)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6">
					<WifiIcon className="h-3.5 w-3.5 text-primary animate-pulse" />
					<span className="text-xs font-medium text-primary">Live</span>
				</div>
			)}
			{awareness && connectionStatus === "reconnecting" && (
				<div className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--yellow-500)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6">
					<WifiIcon className="h-3.5 w-3.5 text-yellow-500 animate-pulse" />
					<span className="text-xs font-medium text-yellow-500">
						Reconnecting...
					</span>
				</div>
			)}
			{awareness && connectionStatus === "disconnected" && (
				<button
					type="button"
					onClick={() => reconnect()}
					className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--destructive)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6 hover:bg-[color-mix(in_oklch,var(--background)_85%,transparent)] transition-colors cursor-pointer"
				>
					<WifiOffIcon className="h-3.5 w-3.5 text-destructive" />
					<span className="text-xs font-medium text-destructive">
						Disconnected - Click to reconnect
					</span>
				</button>
			)}
			{!awareness && (
				<div className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--muted-foreground)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6">
					<WifiOffIcon className="h-3.5 w-3.5 text-muted-foreground" />
					<span className="text-xs font-medium text-muted-foreground">
						Offline
					</span>
				</div>
			)}
			<div className="flex items-center justify-center absolute translate-x-[-50%] mt-5 left-[50dvw] z-40">
				{board.data && editBoard && (
					<BoardMeta
						appId={appId}
						board={board.data}
						boardId={boardId}
						closeMeta={() => setEditBoard(false)}
						version={version}
						selectVersion={(version) => setVersion(version)}
					/>
				)}
				<FlowDock
					mobileClassName="hidden"
					items={[
						...(typeof parentRegister.boardParents[boardId] === "string" &&
						!currentLayer
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
							title: "Manage Board",
							onClick: async () => {
								setEditBoard(true);
							},
						},
						{
							icon: <HistoryIcon />,
							separator: "left",
							title: "Run History",
							onClick: async () => {
								toggleRunHistory();
							},
						},
						...(currentMetadata
							? [
									{
										icon: <ScrollIcon />,
										title: "Logs",
										onClick: async () => {
											toggleLogs();
										},
									},
								]
							: ([] as any)),
						...(currentLayer
							? [
									{
										icon: <SquareChevronUpIcon />,
										title: "Layer Up",
										separator: "left",
										highlight: true,
										onClick: async () => {
											popLayer();
										},
									},
								]
							: []),
					]}
				/>
			</div>
			<ResizablePanelGroup
				direction="horizontal"
				className="flex grow min-h-[calc(100dvh-var(--mobile-header-height,56px)-env(safe-area-inset-bottom))] h-[calc(100dvh-var(--mobile-header-height,56px)-env(safe-area-inset-bottom))] md:min-h-dvh md:h-dvh overscroll-contain"
				style={{
					touchAction: "manipulation",
					WebkitOverflowScrolling: "touch",
					overflow: "hidden",
				}}
			>
				{/* Desktop/Tablet side panels */}
				<ResizablePanel
					className="z-50 bg-background hidden md:block"
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
					<ResizablePanelGroup
						direction="vertical"
						className="h-full flex grow"
					>
						<ResizablePanel autoSave="flow-main" ref={flowPanelRef}>
							<FlowContextMenu
								board={board.data}
								droppedPin={droppedPin}
								onCommentPlace={onCommentPlace}
								refs={board.data?.refs || {}}
								onClose={() => setDroppedPin(undefined)}
								nodes={catalog.data ?? []}
								onPlaceholder={async (name) => {
									await placePlaceholder(name);
									setDroppedPin(undefined);
								}}
								onNodePlace={async (node) => {
									await placeNode(node);
								}}
							>
								<div
									className={`w-full h-full relative select-none touch-none ${isOver && "border-green-400 border-2 z-10"}`}
									ref={setNodeRef}
									style={{
										WebkitUserSelect: "none",
										WebkitTouchCallout: "none",
										touchAction: "none",
									}}
									onTouchStart={(e) => {
										const t = e.touches[0];
										if (!t) return;
										const target = e.currentTarget;
										const startX = t.clientX;
										const startY = t.clientY;
										let moved = false;
										const onMove = (me: TouchEvent) => {
											const tt = me.touches[0];
											if (!tt) return;
											if (
												Math.hypot(tt.clientX - startX, tt.clientY - startY) >
												10
											)
												moved = true;
										};
										const timer = setTimeout(() => {
											if (moved) return;
											// Synthesize a contextmenu-like event for long-press
											const evt = new MouseEvent("contextmenu", {
												clientX: startX,
												clientY: startY,
												bubbles: true,
												cancelable: true,
											});
											target.dispatchEvent(evt);
										}, 450);
										const onEnd = () => {
											clearTimeout(timer);
											document.removeEventListener("touchmove", onMove, {
												capture: true,
											} as any);
											document.removeEventListener("touchend", onEnd, {
												capture: true,
											} as any);
											document.removeEventListener("touchcancel", onEnd, {
												capture: true,
											} as any);
										};
										document.addEventListener("touchmove", onMove, {
											passive: true,
											capture: true,
										} as any);
										document.addEventListener("touchend", onEnd, {
											passive: true,
											capture: true,
										} as any);
										document.addEventListener("touchcancel", onEnd, {
											passive: true,
											capture: true,
										} as any);
									}}
								>
									{currentLayer && (
										<h2 className="absolute bottom-0 left-0 z-10 ml-16 mb-10 text-muted pointer-events-none select-none">
											{board.data?.layers[currentLayer]?.name}
										</h2>
									)}
									{version && (
										<h3 className="absolute top-0 mr-2 mt-2 right-0 z-10 text-muted pointer-events-none select-none">
											Version {version[0]}.{version[1]}.{version[2]} - Read-Only
										</h3>
									)}
									<ReactFlow
										suppressHydrationWarning
										onContextMenu={onContextMenuCB}
										nodesDraggable={typeof version === "undefined"}
										nodesConnectable={typeof version === "undefined"}
										onInit={initializeFlow}
										ref={flowRef}
										colorMode={colorMode}
										nodes={nodes}
										nodeTypes={nodeTypes}
										edges={edges}
										edgeTypes={edgeTypes}
										maxZoom={3}
										minZoom={0.1}
										onNodeDoubleClick={onNodeDoubleClick}
										onNodesChange={onNodesChangeIntercept}
										onEdgesChange={onEdgesChange}
										onNodeDragStop={onNodeDragStop}
										onNodeDrag={onNodeDrag}
										isValidConnection={isValidConnectionCB}
										onConnect={onConnect}
										onSelectionChange={onSelectionChange}
										onReconnect={onReconnect}
										onReconnectStart={onReconnectStart}
										onMoveEnd={onMoveEnd}
										// onEdgeDoubleClick={(e, edge) => {
										// 	console.dir({e, edge})
										// }}
										onReconnectEnd={onReconnectEnd}
										onConnectEnd={onConnectEnd}
										fitView
										proOptions={{ hideAttribution: true }}
									>
										<Controls />
										<MiniMap
											pannable
											zoomable
											bgColor="color-mix(in oklch, var(--background) 80%, transparent)"
											maskColor="color-mix(in oklch, var(--foreground) 10%, transparent)"
											nodeColor={(node) => {
												if (node.type === "layerNode")
													return "color-mix(in oklch, var(--foreground) 50%, transparent)";

												if (node.type === "node") {
													const nodeData: INode = node.data.node as INode;
													if (nodeData.event_callback)
														return "color-mix(in oklch, var(--primary) 80%, transparent)";
													if (nodeData.start)
														return "color-mix(in oklch, var(--primary) 80%, transparent)";
													if (
														!Object.values(nodeData.pins).find(
															(pin) =>
																pin.data_type === IVariableType.Execution,
														)
													) {
														return "color-mix(in oklch, var(--tertiary) 80%, transparent)";
													}
													return "color-mix(in oklch, var(--muted) 80%, transparent)";
												}
												if (node.type === "commentNode") {
													const commentData: IComment = node.data
														.comment as IComment;
													let color =
														commentData.color ??
														"color-mix(in oklch, var(--muted) 80%, transparent)";

													if (color.startsWith("#")) {
														color = hexToRgba(color, 0.3);
													}
													return color;
												}
												return "color-mix(in oklch, var(--primary) 60%, transparent)";
											}}
										/>
										<Background
											variant={
												currentLayer
													? BackgroundVariant.Lines
													: BackgroundVariant.Dots
											}
											color={
												currentLayer
													? "color-mix(in oklch, var(--foreground) 5%, transparent)"
													: "color-mix(in oklch, var(--foreground) 20%, transparent)"
											}
											bgColor="color-mix(in oklch, var(--background) 80%, transparent)"
											gap={12}
											size={1}
										/>
									</ReactFlow>
									{peerStates.length > 0 && (
										<FlowCursors
											peers={peerStates}
											currentLayerPath={layerPath ?? "root"}
										/>
									)}
									{peerStates.length > 0 && (
										<FlowLayerIndicators
											peers={peerStates}
											currentLayerPath={layerPath ?? "root"}
											nodes={nodes}
										/>
									)}
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
							className="z-50 hidden md:block"
							hidden={!currentMetadata}
							ref={logPanelRef}
							defaultSize={0}
							collapsedSize={0}
							collapsible={true}
							autoSave="flow-logs"
						>
							{board.data && currentMetadata && (
								<Traces
									appId={appId}
									boardId={boardId}
									board={boardRef}
									onFocusNode={(nodeId: string) => {
										focusNode(nodeId);
									}}
								/>
							)}
						</ResizablePanel>
					</ResizablePanelGroup>
				</ResizablePanel>
				<ResizableHandle withHandle />
				<ResizablePanel
					className="z-50 hidden md:block"
					autoSave="flow-runs"
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
							version={board.data.version as [number, number, number]}
							onVersionChange={setVersion}
						/>
					)}
				</ResizablePanel>
				{/* Mobile sheets */}
				<Sheet open={varsOpen} onOpenChange={setVarsOpen}>
					<SheetContent side="bottom" className="h-[60dvh] w-full">
						<SheetHeader>
							<SheetTitle>Variables</SheetTitle>
						</SheetHeader>
						{board.data && (
							<div className="h-[calc(60dvh-3.5rem)] overflow-y-auto overscroll-contain">
								<VariablesMenu
									board={board.data}
									executeCommand={executeCommand}
								/>
							</div>
						)}
					</SheetContent>
				</Sheet>
				<Sheet open={runsOpen} onOpenChange={setRunsOpen}>
					<SheetContent side="bottom" className="h-[80dvh] w-full">
						<SheetHeader>
							<SheetTitle>Runs</SheetTitle>
						</SheetHeader>
						{board.data && (
							<div className="h-[calc(80dvh-3.5rem)] overflow-y-auto overscroll-contain">
								<FlowRuns
									executeBoard={executeBoard}
									nodes={board.data.nodes}
									appId={appId}
									boardId={boardId}
									version={board.data.version as [number, number, number]}
									onVersionChange={setVersion}
								/>
							</div>
						)}
					</SheetContent>
				</Sheet>
				<Sheet open={logsOpen} onOpenChange={setLogsOpen}>
					<SheetContent side="bottom" className="h-[80dvh] w-full">
						<SheetHeader>
							<SheetTitle>Logs</SheetTitle>
						</SheetHeader>
						{board.data && currentMetadata && (
							<div className="h-[calc(80dvh-3.5rem)] w-full">
								<Traces
									appId={appId}
									boardId={boardId}
									board={boardRef}
									onFocusNode={(nodeId: string) => {
										focusNode(nodeId);
									}}
								/>
							</div>
						)}
						{(!currentMetadata || !board.data) && (
							<div className="h-[calc(80dvh-3.5rem)] w-full flex items-center justify-center text-sm text-muted-foreground p-6">
								No run selected yet. Start a run to view logs here.
							</div>
						)}
					</SheetContent>
				</Sheet>
			</ResizablePanelGroup>
			<PinEditModal appId={appId} boardId={boardId} version={version} />
			<FlowNodeInfoOverlay
				key={boardId}
				ref={nodeInfoOverlayRef}
				refs={board.data?.refs}
				boardRef={boardRef}
				onFocusNode={focusNode}
			/>
		</div>
	);
}
