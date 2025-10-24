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
	type OnSelectionChangeFunc,
	type OnNodesChange,
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
	Redo2Icon,
	ScrollIcon,
	SquareChevronUpIcon,
	Undo2Icon,
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
	viewportDb,
	viewportKey,
} from "../..";
import { CommentNode } from "../../components/flow/comment-node";
import { FlowContextMenu } from "../../components/flow/flow-context-menu";
import { FlowDock } from "../../components/flow/flow-dock";
import { FlowNode, type RemoteSelectionParticipant } from "../../components/flow/flow-node";
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
import { useInvoke } from "../../hooks/use-invoke";
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
	removeLayerCommand,
	removeNodeCommand,
	upsertCommentCommand,
	upsertLayerCommand,
} from "../../lib";
import {
	handleCopy,
	handlePaste,
	isValidConnection,
	parseBoard,
} from "../../lib/flow-board-utils";
import { toastError, toastSuccess } from "../../lib/messages";
import {
	type IBoard,
	type IComment,
	ICommentType,
	type IVariable,
} from "../../lib/schema/flow/board";
import { ILayerType } from "../../lib/schema/flow/board/commands/upsert-layer";
import { type INode, IVariableType } from "../../lib/schema/flow/node";
import type { IPin } from "../../lib/schema/flow/pin";
import type { ILayer } from "../../lib/schema/flow/run";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { useBackend, useBackendStore } from "../../state/backend-state";
import { useFlowBoardParentState } from "../../state/flow-board-parent-state";
import { useRunExecutionStore } from "../../state/run-execution-state";
import { BoardMeta } from "./board-meta";
import { useUndoRedo } from "./flow-history";
import { PinEditModal } from "./flow-pin/edit-modal";
import { FlowRuns } from "./flow-runs";
import { LayerInnerNode } from "./layer-inner-node";
import { LayerNode } from "./layer-node";
import { createRealtimeSession } from "../../lib";
import { FlowCursors } from "./flow-cursors";
import { FlowLayerIndicators } from "./flow-layer-indicators";

function hexToRgba(hex: string, alpha = 0.3): string {
	let c = hex.replace("#", "");
	if (c.length === 3) c = c[0] + c[0] + c[1] + c[1] + c[2] + c[2];
	const num = Number.parseInt(c, 16);
	const r = (num >> 16) & 255;
	const g = (num >> 8) & 255;
	const b = num & 255;
	return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

interface PeerPresence {
	clientId: number;
	cursor?: { x: number; y: number };
	user: { id?: string; name: string; color: string; avatar?: string };
	layerPath: string;
	selection: { nodes: string[] };
}

function normalizeSelectionNodes(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	return value.filter((nodeId: unknown): nodeId is string => typeof nodeId === "string");
}

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
	const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });
	const [pinCache, setPinCache] = useState<
		Map<string, [IPin, INode | ILayer, boolean]>
	>(new Map());
	const [editBoard, setEditBoard] = useState(false);
	const [currentLayer, setCurrentLayer] = useState<string | undefined>();
	const [layerPath, setLayerPath] = useState<string | undefined>();
	const [peerStates, setPeerStates] = useState<PeerPresence[]>([]);
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

	const saveViewport = useCallback(async () => {
		try {
			const vp = getViewport();
			await viewportDb.viewports.put({
				id: viewportKey(appId, boardId, layerPath),
				appId,
				boardId,
				layerPath: layerPath ?? "root",
				x: vp.x,
				y: vp.y,
				zoom: vp.zoom,
				updatedAt: Date.now(),
			});
		} catch {
			// no-op
		}
	}, [appId, boardId, layerPath, getViewport]);

	useEffect(() => {
		let active = true;

		const restore = async () => {
			const rec = await viewportDb.viewports.get(
				viewportKey(appId, boardId, layerPath),
			);
			if (!active) return;

			if (rec) {
				setViewport({ x: rec.x, y: rec.y, zoom: rec.zoom });
			} else {
				// Fit screen when no stored viewport is found
				fitView({ duration: 300 });
			}
		};

		// Wait until nodes are there so fitting has effect
		// Using nodes.length is enough to re-run after initial load
		restore();

		return () => {
			active = false;
		};
	}, [appId, boardId, layerPath, setViewport, fitView, nodes.length]);

	const focusNode = useCallback(
		(nodeId: string) => {
			const node = board.data?.nodes[nodeId];
			if (!node) {
				console.error("Node not found:", nodeId);
				return;
			}

			const layers = board.data?.layers ?? {};
			const layerTree: string[] = [];
			let parentLayer = node.layer;
			let iteration = 0;

			while (parentLayer && iteration < 40) {
				iteration++;
				const layer = layers[parentLayer];
				if (!layer) break;
				layerTree.push(layer.id);
				parentLayer = layer.parent_id;
			}

			if (layerTree.length > 0) {
				setCurrentLayer(layerTree[layerTree.length - 1]);
				setLayerPath(layerTree.slice().reverse().join("/"));
			} else {
				setCurrentLayer(undefined);
				setLayerPath(undefined);
			}

			// Defer until layer switch renders nodes
			requestAnimationFrame(() => {
				requestAnimationFrame(() => {
					fitView({
						nodes: [{ id: node.id }],
						padding: 3,
						duration: 500,
					});
				});
			});
		},
		[board.data?.nodes, board.data?.layers, fitView],
	);

	const executeCommand = useCallback(
		async (command: IGenericCommand, append = false): Promise<any> => {
			const backend = useBackendStore.getState().backend;
			if (!backend) return;
			if (typeof version !== "undefined") {
				toastError("Cannot change old version", <XIcon />);
				return;
			}
			const result = await backend.boardState.executeCommand(
				appId,
				boardId,
				command,
			);
			await pushCommand(result, append);
			await board.refetch();

			// Broadcast board update to peers
			if (awarenessRef.current) {
				awarenessRef.current.setLocalStateField("boardUpdate", Date.now());
			}

			return result;
		},
		[board.refetch, appId, boardId, pushCommand, version],
	);

	const executeCommands = useCallback(
		async (commands: IGenericCommand[]) => {
			const backend = useBackendStore.getState().backend;
			if (!backend) return;
			if (typeof version !== "undefined") {
				toastError("Cannot change old version", <XIcon />);
				return;
			}
			if (commands.length === 0) return;
			const result = await backend.boardState.executeCommands(
				appId,
				boardId,
				commands,
			);
			await pushCommands(result);
			await board.refetch();

			// Broadcast board update to peers
			if (awarenessRef.current) {
				awarenessRef.current.setLocalStateField("boardUpdate", Date.now());
			}

			return result;
		},
		[board.refetch, appId, boardId, pushCommands, version],
	);

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

	function toggleVars() {
		// On mobile use sheet instead of resizable panel
		if (
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches
		) {
			setVarsOpen((v) => !v);
			return;
		}
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
		if (
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches
		) {
			setRunsOpen((v) => !v);
			return;
		}
		if (!runsPanelRef.current) return;
		const isCollapsed = runsPanelRef.current.isCollapsed();
		isCollapsed
			? runsPanelRef.current.expand()
			: runsPanelRef.current.collapse();

		if (!isCollapsed) {
			return;
		}

		const size = runsPanelRef.current.getSize();
		if (size < 10) runsPanelRef.current.resize(30);
	}

	function toggleLogs() {
		if (
			typeof window !== "undefined" &&
			window.matchMedia("(max-width: 767px)").matches
		) {
			setLogsOpen((v) => !v);
			return;
		}
		if (!logPanelRef.current) return;
		const isCollapsed = logPanelRef.current.isCollapsed();
		isCollapsed ? logPanelRef.current.expand() : logPanelRef.current.collapse();

		if (!isCollapsed) {
			return;
		}

		const size = logPanelRef.current.getSize();
		if (size < 10) logPanelRef.current.resize(20);
	}

	const pushLayer = useCallback(
		(pushedLayer: ILayer) => {
			void saveViewport();

			setCurrentLayer(pushedLayer.id);
			setLayerPath((old) => {
				if (old) return `${old}/${pushedLayer.id}`;
				return pushedLayer.id;
			});
		},
		[saveViewport],
	);

	const popLayer = useCallback(() => {
		if (!layerPath) return;

		// Save current layer viewport before switching
		void saveViewport();

		const segments = layerPath.split("/");
		if (segments.length === 1) {
			setLayerPath(undefined);
			setCurrentLayer(undefined);
			return;
		}
		const newPath = segments.slice(0, -1).join("/");
		setLayerPath(newPath);
		const segment = newPath.split("/").pop();
		setCurrentLayer(segment);
	}, [layerPath, saveViewport]);

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
			const target = event.target as HTMLElement;
			if (
				target.tagName === "INPUT" ||
				target.tagName === "TEXTAREA" ||
				target.isContentEditable
			) {
				return;
			}
			// Undo
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "z" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const stack = await undo();
				if (stack) await backend.boardState.undoBoard(appId, boardId, stack);
				toastSuccess("Undo", <Undo2Icon className="w-4 h-4" />);
				await board.refetch();
				return;
			}

			// Redo
			if ((event.metaKey || event.ctrlKey) && event.key === "y") {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const stack = await redo();
				if (stack) await backend.boardState.redoBoard(appId, boardId, stack);
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
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
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
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
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
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
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
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const node = catalog.data?.find((node) => node.name === "reroute");
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
			}
		},
		[boardId, board, backend, version],
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
				current_layer: currentLayer,
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
							pin.data_type !== IVariableType.Generic &&
							droppedPin.data_type !== IVariableType.Generic
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
							(options?.enforce_generic_value_type ?? false) ||
							(pin.options?.enforce_generic_value_type ?? false)
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
		[
			clickPosition,
			boardId,
			droppedPin,
			board.data?.refs,
			currentLayer,
			screenToFlowPosition,
		],
	);

	const placePlaceholder = useCallback(
		async (name: string, position?: { x: number; y: number }) => {
			const delayNode = catalog.data?.find((node) => node.name === "delay");

			const refs = board.data?.refs ?? {};
			const location = screenToFlowPosition({
				x: position?.x ?? clickPosition.x,
				y: position?.y ?? clickPosition.y,
			});

			const layerId = createId();

			const execInPin: IPin = {
				id: createId(),
				name: "exec_in",
				friendly_name: "Exec In",
				connected_to: [],
				depends_on: [],
				description: "",
				index: 1,
				pin_type: IPinType.Input,
				value_type: IValueType.Normal,
				data_type: IVariableType.Execution,
				default_value: null,
			};

			const execOutPin: IPin = {
				...execInPin,
				id: createId(),
				pin_type: IPinType.Output,
				name: "exec_out",
				friendly_name: "Exec Out",
				index: 1,
			};

			let dataPin: IPin | undefined;
			let connectToPinId: string | undefined;

			if (droppedPin) {
				const oppositeType =
					droppedPin.pin_type === "Input" ? IPinType.Output : IPinType.Input;

				if (droppedPin.data_type === IVariableType.Execution) {
					connectToPinId =
						oppositeType === IPinType.Input ? execInPin.id : execOutPin.id;
				} else {
					const resolvedSchema =
						typeof droppedPin.schema === "string"
							? (refs?.[droppedPin.schema] ?? droppedPin.schema)
							: droppedPin.schema;

					dataPin = {
						id: createId(),
						name: oppositeType === IPinType.Input ? "in" : "out",
						friendly_name: oppositeType === IPinType.Input ? "In" : "Out",
						connected_to: [],
						depends_on: [],
						description: "",
						index: 2,
						pin_type: oppositeType,
						value_type: droppedPin.value_type,
						data_type: droppedPin.data_type,
						default_value: null,
						...(resolvedSchema ? { schema: resolvedSchema } : {}),
						...(droppedPin.options ? { options: droppedPin.options } : {}),
					};

					connectToPinId = dataPin.id;
				}
			}

			const pins: Record<string, IPin> = {
				[execInPin.id]: execInPin,
				[execOutPin.id]: execOutPin,
				...(dataPin ? { [dataPin.id]: dataPin } : {}),
			};

			const newLayerCommand = upsertLayerCommand({
				current_layer: currentLayer,
				layer: {
					comments: {},
					coordinates: [location.x, location.y, 0],
					id: layerId,
					name,
					nodes: {},
					pins,
					type: ILayerType.Collapsed,
					variables: {},
					parent_id: currentLayer,
				},
				node_ids: [],
			});

			const newLayerResult = await executeCommand(newLayerCommand, false);
			const newLayer: ILayer = newLayerResult.layer;

			if (delayNode) {
				const placeDelayCommand = addNodeCommand({
					node: delayNode,
					current_layer: newLayer.id,
				});

				const placedNode = await executeCommand(placeDelayCommand.command);
				const newNode: INode = placedNode.node;
				const newNodeInPin = Object.values(newNode.pins).find(
					(pin) =>
						pin.pin_type === IPinType.Input &&
						pin.data_type === IVariableType.Execution,
				);
				const newNodeOutPin = Object.values(newNode.pins).find(
					(pin) =>
						pin.pin_type === IPinType.Output &&
						pin.data_type === IVariableType.Execution,
				);

				const connectOutput = connectPinsCommand({
					from_node: newNode.id,
					from_pin: newNodeOutPin!.id,
					to_node: newLayer.id,
					to_pin: execOutPin.id,
				});

				const connectInput = connectPinsCommand({
					to_node: newNode.id,
					to_pin: newNodeInPin!.id,
					from_node: newLayer.id,
					from_pin: execInPin.id,
				});

				await executeCommands([connectOutput, connectInput]);
			}

			if (!droppedPin) {
				return;
			}
			const pinType = droppedPin.pin_type === "Input" ? "Output" : "Input";
			const pinValueType = droppedPin.value_type;
			const pinDataType = droppedPin.data_type;
			const options = droppedPin.options;

			const pin = Object.values(newLayer.pins).find((pin) => {
				if (pin.pin_type !== pinType) false;
				if (pin.value_type !== pinValueType) {
					if (
						pinDataType !== IVariableType.Generic &&
						pin.data_type !== IVariableType.Generic
					)
						return false;
					if (
						(options?.enforce_generic_value_type ?? false) ||
						(pin.options?.enforce_generic_value_type ?? false)
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
			if (!sourcePin || !sourceNode) {
				return;
			}
			if (!pin) {
				return;
			}

			const command = connectPinsCommand({
				from_node:
					droppedPin.pin_type === "Output" ? sourceNode.id : newLayer.id,
				from_pin: droppedPin.pin_type === "Output" ? sourcePin.id : pin?.id,
				to_node: droppedPin.pin_type === "Input" ? sourceNode.id : newLayer.id,
				to_pin: droppedPin.pin_type === "Input" ? sourcePin.id : pin?.id,
			});

			await executeCommand(command);
		},
		[
			clickPosition,
			boardId,
			droppedPin,
			board.data?.refs,
			executeCommand,
			pinCache,
			currentLayer,
			screenToFlowPosition,
		],
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

	// Realtime session
	const [awareness, setAwareness] = useState<any | undefined>(undefined);
	const awarenessRef = useRef<any | undefined>(undefined);
	const [connectionStatus, setConnectionStatus] = useState<'connected' | 'disconnected' | 'reconnecting'>('disconnected');
	const sessionRef = useRef<{ dispose: () => void; reconnect: () => Promise<void> } | null>(null);
	const hasBoardData = !!board.data;

	useEffect(() => {
		let disposed = false;
		const setup = async () => {
			try {
				// Check offline status first
				const offline = await backend.isOffline(appId);
				console.log("[FlowBoard] Offline status:", offline, "Version:", version, "Board data:", hasBoardData);

				// Only enable realtime on latest/editable versions
				if (!hasBoardData || typeof version !== "undefined") return;
				if (offline) return;
				const room = `${appId}:${boardId}`;
				const [access, jwks] = await Promise.all([
					backend.boardState.getRealtimeAccess(appId, boardId),
					backend.boardState.getRealtimeJwks(appId, boardId).catch(() => undefined as any),
				]);
				const name = currentProfile.data?.name || currentProfile.data?.settings?.display_name || "Anonymous";
				const userId = currentProfile.data?.id;

				const session = await createRealtimeSession({
					room,
					access,
					jwks,
					name,
					userId,
					signalingServers: hub.hub?.signaling ?? [],
					onStatusChange: (status) => {
						console.log(`[FlowBoard] Connection status changed: ${status}`);
						setConnectionStatus(status);
					}
				});
				if (disposed) { session.dispose(); return; }
				sessionRef.current = { dispose: session.dispose, reconnect: session.reconnect };
				awarenessRef.current = session.awareness;
				setAwareness(session.awareness);
				setConnectionStatus('connected');
			} catch (e) {
				console.warn("Realtime setup failed:", e);
				setConnectionStatus('disconnected');
			}
		};
		void setup();
		return () => {
			disposed = true;
			try { sessionRef.current?.dispose(); } catch {}
			sessionRef.current = null;
			awarenessRef.current = undefined;
			setAwareness(undefined);
			setConnectionStatus('disconnected');
		};
	}, [backend, appId, boardId, hasBoardData, version, currentProfile.data?.id, currentProfile.data?.name, hub.hub]);

	useEffect(() => {
		if (!awareness) {
			setPeerStates([]);
			return;
		}

		const updatePeers = () => {
			const states = awareness.getStates() as Map<number, any>;
			const invalidPeers: Set<number> | undefined = (awareness as any)?.__invalidPeers;
			const next: PeerPresence[] = [];
			states.forEach((state, clientId) => {
				const isSelf = clientId === awareness.clientID;
				const isInvalid = invalidPeers?.has(clientId) ?? false;
				if (isSelf || isInvalid) return;
				const user = state?.user ?? {};
				const cursor = state?.cursor;
				next.push({
					clientId,
					cursor: cursor ? { x: cursor.x, y: cursor.y } : undefined,
					user: {
						id: user?.id,
						name: user?.name ?? "User",
						color: user?.color ?? "#22c55e",
						avatar: user?.avatar,
					},
					layerPath: state?.layerPath ?? "root",
					selection: { nodes: normalizeSelectionNodes(state?.selection?.nodes) },
				});
			});
			setPeerStates(next);
		};

		const handleChange = () => updatePeers();
		awareness.on("change", handleChange);
		updatePeers();
		return () => {
			try {
				awareness.off("change", handleChange);
			} catch {}
		};
	}, [awareness]);

	// Listen for peer board updates and refetch
	useEffect(() => {
		if (!awareness) return;

		const handleBoardUpdate = ({ added, updated }: { added: number[]; updated: number[] }) => {
			const states = awareness.getStates() as Map<number, any>;
			const changedPeers = [...added, ...updated];

			for (const clientId of changedPeers) {
				if (clientId === awareness.clientID) continue;
				const state = states.get(clientId);
				if (state?.boardUpdate) {
					// Peer made a board update, refetch
					void board.refetch();
					break;
				}
			}
		};

		awareness.on("update", handleBoardUpdate);
		return () => {
			try {
				awareness.off("update", handleBoardUpdate);
			} catch {}
		};
	}, [awareness, board]);

	// Broadcast cursor position via awareness
	useEffect(() => {
		if (!awareness) return;
		const flowPoint = screenToFlowPosition({
			x: mousePosition.x,
			y: mousePosition.y,
		});
		awareness.setLocalStateField("cursor", {
			x: flowPoint.x,
			y: flowPoint.y,
		});
	}, [mousePosition.x, mousePosition.y, awareness, screenToFlowPosition]);

	// Broadcast current layer path via awareness
	useEffect(() => {
		if (!awareness) return;
		awareness.setLocalStateField("layerPath", layerPath ?? "root");
	}, [awareness, layerPath]);

	useEffect(() => {
		if (!awareness) return;
		awareness.setLocalStateField("selection", { nodes: [] });
	}, [awareness]);

	useEffect(() => {
		if (!awareness) return;
		const profileName =
			currentProfile.data?.name ?? currentProfile.data?.settings?.display_name;
		if (!profileName && !currentProfile.data?.id) return;
		const localUser = awareness.getLocalState()?.user ?? {};
		awareness.setLocalStateField("user", {
			...localUser,
			name: profileName ?? localUser.name ?? "Anonymous",
			id: currentProfile.data?.id ?? localUser.id,
		});
	}, [awareness, currentProfile.data?.id, currentProfile.data?.name, currentProfile.data?.settings?.display_name]);

	// Use ref to track remote selections and update nodes only when necessary
	const remoteSelectionsRef = useRef<Map<string, RemoteSelectionParticipant[]>>(new Map());

	useEffect(() => {
		const map = new Map<string, RemoteSelectionParticipant[]>();
		for (const peer of peerStates) {
			if (!peer.selection.nodes.length) continue;
			for (const nodeId of peer.selection.nodes) {
				if (!nodeId) continue;
				const participant: RemoteSelectionParticipant = {
					clientId: peer.clientId,
					userId: peer.user.id,
					name: peer.user.name,
					color: peer.user.color,
				};
				const existing = map.get(nodeId) ?? [];
				map.set(nodeId, [...existing, participant]);
			}
		}
		map.forEach((participants, key) => {
			map.set(
				key,
				participants
					.slice()
					.sort((a, b) =>
						a.clientId === b.clientId
							? a.name.localeCompare(b.name)
							: a.clientId - b.clientId,
					),
			);
		});

		// Check if selections actually changed
		let hasChanges = false;
		if (map.size !== remoteSelectionsRef.current.size) {
			hasChanges = true;
		} else {
			for (const [nodeId, participants] of map.entries()) {
				const prev = remoteSelectionsRef.current.get(nodeId);
				if (!prev || prev.length !== participants.length) {
					hasChanges = true;
					break;
				}
				for (let i = 0; i < participants.length; i++) {
					const p = participants[i];
					const prevP = prev[i];
					if (!prevP || p.clientId !== prevP.clientId || p.userId !== prevP.userId ||
						p.name !== prevP.name || p.color !== prevP.color) {
						hasChanges = true;
						break;
					}
				}
				if (hasChanges) break;
			}
		}

		if (!hasChanges) return;

		remoteSelectionsRef.current = map;

		// Only update nodes when selections changed
		setNodes((nds) => {
			if (nds.length === 0) return nds;
			const updated = nds.map((node) => {
				if (node.type !== "node") return node;
				const participants = map.get(node.id) ?? [];
				const hasSelections = participants.length > 0;
				const hadSelections = !!node.data.remoteSelections && node.data.remoteSelections.length > 0;

				if (!hasSelections && !hadSelections) return node;

				return {
					...node,
					data: {
						...node.data,
						remoteSelections: hasSelections ? participants : undefined
					},
				};
			});
			return updated;
		});
	}, [peerStates, setNodes]);

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
		);

		setNodes(parsed.nodes);
		setEdges(parsed.edges);
		setPinCache(new Map(parsed.cache));
	}, [board.data, currentLayer, currentProfile.data, version]);

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

	const onConnect = useCallback(
		(params: any) =>
			setEdges((eds) => {
				// Don't execute commands when viewing an old version
				if (typeof version !== "undefined") {
					return eds;
				}

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
		[setEdges, pinCache, boardId, version, executeCommand],
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

				// Don't execute commands when viewing an old version
				if (typeof version !== "undefined") {
					return applyNodeChanges(changes, nds);
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

							const foundLayer = Object.values(board.data?.layers || {}).find(
								(layer) => layer.id === change.id,
							);

							if (foundLayer) {
								return removeLayerCommand({
									child_layers: [],
									layer: foundLayer,
									layer_nodes: [],
									layers: [],
									nodes: [],
									preserve_nodes: false,
								});
							}

							return undefined;
						})
						.filter((command) => command !== undefined) as any[],
				);

				return applyNodeChanges(changes, nds);
			}),
		[setNodes, board.data, boardId, executeCommands, version],
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

				// Don't execute commands when viewing an old version
				if (typeof version !== "undefined") {
					return applyEdgeChanges(changes, eds);
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

							return disconnectPinsCommand({
								from_node: fromNode.id,
								from_pin: fromPin.id,
								to_node: toNode.id,
								to_pin: toPin.id,
							});
						})
						.filter((command: any) => command !== undefined) as any[],
				);

				return applyEdgeChanges(changes, eds);
			}),
		[setEdges, board.data, boardId, pinCache, executeCommands, version],
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
			if (oldEdge.id === new_id) return;

			const oldEdgeToNode = pinToNode(oldEdge.targetHandle);
			const oldEdgeFromNode = pinToNode(oldEdge.sourceHandle);

			if (!oldEdgeToNode || !oldEdgeFromNode) return;

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

			edgeReconnectSuccessful.current = true;
			setEdges((els) => reconnectEdge(oldEdge, newConnection, els));
		},
		[setEdges, pinToNode, executeCommands, version],
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

	return (
		<div className="w-full flex-1 grow flex-col min-h-0 relative">
			{/* Realtime connection status indicator */}
			{awareness && connectionStatus === 'connected' && (
				<div className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--primary)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6">
					<WifiIcon className="h-3.5 w-3.5 text-primary animate-pulse" />
					<span className="text-xs font-medium text-primary">Live</span>
				</div>
			)}
			{awareness && connectionStatus === 'reconnecting' && (
				<div className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--yellow-500)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6">
					<WifiIcon className="h-3.5 w-3.5 text-yellow-500 animate-pulse" />
					<span className="text-xs font-medium text-yellow-500">Reconnecting...</span>
				</div>
			)}
			{awareness && connectionStatus === 'disconnected' && (
				<button
					type="button"
					onClick={() => sessionRef.current?.reconnect()}
					className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--destructive)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6 hover:bg-[color-mix(in_oklch,var(--background)_85%,transparent)] transition-colors cursor-pointer"
				>
					<WifiOffIcon className="h-3.5 w-3.5 text-destructive" />
					<span className="text-xs font-medium text-destructive">Disconnected - Click to reconnect</span>
				</button>
			)}
			{!awareness && (
				<div className="fixed right-3 top-16 z-50 flex items-center gap-2 rounded-xl border border-[color-mix(in_oklch,var(--muted-foreground)_35%,transparent)] bg-[color-mix(in_oklch,var(--background)_92%,transparent)] px-3 py-1.5 backdrop-blur-sm shadow-sm sm:right-4 sm:top-16 md:right-6 md:top-6">
					<WifiOffIcon className="h-3.5 w-3.5 text-muted-foreground" />
					<span className="text-xs font-medium text-muted-foreground">Offline</span>
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
									{peerStates.length > 0 && <FlowCursors peers={peerStates} currentLayerPath={layerPath ?? "root"} />}
									{peerStates.length > 0 && <FlowLayerIndicators peers={peerStates} currentLayerPath={layerPath ?? "root"} nodes={nodes} />}
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
		</div>
	);
}
