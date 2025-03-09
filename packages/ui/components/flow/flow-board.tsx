"use client";
import { DragOverlay, useDroppable } from "@dnd-kit/core";
import { createId } from "@paralleldrive/cuid2";
import { type UseQueryResult } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
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
  reconnectEdge,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  ArrowBigLeftDashIcon,
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
import { type ImperativePanelHandle } from "react-resizable-panels";
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
import { useInvoke } from "../../hooks/use-invoke";
import {
  handleCopy,
  handlePaste,
  isValidConnection,
  parseBoard,
} from "../../lib/flow-board";
import { toastError, toastSuccess } from "../../lib/messages";
import {
  type IBoard,
  type IComment,
  ICommentType,
  IExecutionStage,
  ILogLevel,
  type IVariable,
} from "../../lib/schema/flow/board";
import { type INode } from "../../lib/schema/flow/node";
import { type IPin } from "../../lib/schema/flow/pin";
import { type IRun, type ITrace } from "../../lib/schema/flow/run";
import { convertJsonToUint8Array } from "../../lib/uint8";
import { useFlowBoardParentState } from "../../state/flow-board-parent-state";
import { useRunExecutionStore } from "../../state/run-execution-state";
import { type ISettingsProfile } from "../../types";
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
export function FlowBoard({ boardId }: Readonly<{ boardId: string }>) {
  const router = useRouter();
  const selected = useRef(new Set<string>());
  const edgeReconnectSuccessful = useRef(true);
  const { isOver, setNodeRef, active } = useDroppable({ id: "flow" });
  const parentRegister = useFlowBoardParentState();

  const flowRef = useRef<any>(null);

  const flowPanelRef = useRef<ImperativePanelHandle>(null);
  const logPanelRef = useRef<ImperativePanelHandle>(null);
  const varPanelRef = useRef<ImperativePanelHandle>(null);

  const { resolvedTheme } = useTheme();

  const catalog: UseQueryResult<INode[]> = useInvoke("get_catalog", {});
  const board: UseQueryResult<IBoard> = useInvoke(
    "get_board",
    { boardId: boardId },
    [boardId],
    boardId !== "",
  );
  const openBoards = useInvoke<[string, string][]>("get_open_boards", {});
  const currentProfile: UseQueryResult<ISettingsProfile | undefined> =
    useInvoke("get_current_profile", {});
  const { addRun, removeRun } = useRunExecutionStore();

  const [traces, setTraces] = useState<
    { node?: INode; traces: ITrace[] } | undefined
  >(undefined);
  const [nodes, setNodes] = useNodesState<any>([]);
  const [edges, setEdges] = useEdgesState<any>([]);
  const [currentRun, setCurrentRun] = useState<IRun | undefined>(undefined);
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

  const colorMode = useMemo(
    () => (resolvedTheme === "dark" ? "dark" : "light"),
    [resolvedTheme],
  );

  const executeCommand = useCallback(
    async (
      command: string,
      args: any,
      append: boolean = false,
    ): Promise<any> => {
      const result = await invoke(command, { ...args, append: append });
      await board.refetch();
      return result;
    },
    [board.refetch],
  );

  useEffect(() => {
    if (!logPanelRef.current) return;

    if (!traces) return logPanelRef.current.collapse();

    logPanelRef.current.expand();
    let size = logPanelRef.current.getSize();

    if (size < 10) logPanelRef.current.resize(45);
  }, [traces, logPanelRef.current]);

  function toggleVars() {
    if (!varPanelRef.current) return;
    const isCollapsed = varPanelRef.current.isCollapsed();
    isCollapsed ? varPanelRef.current.expand() : varPanelRef.current.collapse();

    if (!isCollapsed) {
      return;
    }

    let size = varPanelRef.current.getSize();
    if (size < 10) varPanelRef.current.resize(20);
  }

  const executeBoard = useCallback(
    async (node: INode) => {
      setCurrentRun(undefined);
      const runId: string | undefined = await invoke("create_run", {
        boardId: boardId,
        startIds: [node.id],
      });
      if (!runId) {
        toastError(
          "Failed to execute board",
          <PlayCircleIcon className="w-4 h-4" />,
        );
        return;
      }
      await addRun(runId, boardId, [node.id]);
      await invoke("execute_run", { id: runId });
      removeRun(runId);
      const result: IRun | undefined = await invoke("get_run", { id: runId });
      setCurrentRun(result);
      await invoke("finalize_run", { id: runId });
    },
    [boardId],
  );

  const openTraces = useCallback(async function openTraces(
    node: INode,
    traces: ITrace[],
  ) {
    setTraces({
      node: node,
      traces: traces,
    });
  }, []);

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
      handleCopy(event, nodes);
    },
    [nodes],
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
        await invoke("undo_board", { boardId: boardId });
        toastSuccess("Undo", <Undo2Icon className="w-4 h-4" />);
        await board.refetch();
        return;
      }

      // Redo
      if ((event.metaKey || event.ctrlKey) && event.key === "y") {
        event.preventDefault();
        event.stopPropagation();
        await invoke("redo_board", { boardId: boardId });
        toastSuccess("Redo", <Redo2Icon className="w-4 h-4" />);
        await board.refetch();
      }
    },
    [boardId, board],
  );

  const placeNode = useCallback(
    async (node: INode, position?: { x: number; y: number }) => {
      const location = screenToFlowPosition({
        x: position?.x ?? clickPosition.x,
        y: position?.y ?? clickPosition.y,
      });
      const new_node: INode = await executeCommand("add_node_to_board", {
        boardId: boardId,
        node: { ...node, coordinates: [location.x, location.y, 0] },
      });
      if (droppedPin) {
        const pinType = droppedPin.pin_type === "Input" ? "Output" : "Input";
        const pinDataType = droppedPin.data_type;

        const pin = Object.values(new_node.pins).find(
          (pin) => pin.pin_type === pinType && pin.data_type === pinDataType,
        );
        const [sourcePin, sourceNode] = pinCache.get(droppedPin.id) || [];
        if (!sourcePin || !sourceNode) return;

        await executeCommand("connect_pins", {
          boardId: boardId,
          fromNode:
            droppedPin.pin_type === "Output" ? sourceNode.id : new_node.id,
          fromPin: droppedPin.pin_type === "Output" ? sourcePin.id : pin?.id,
          toNode: droppedPin.pin_type === "Input" ? sourceNode.id : new_node.id,
          toPin: droppedPin.pin_type === "Input" ? sourcePin.id : pin?.id,
        });
      }
    },
    [clickPosition, boardId, droppedPin],
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

    const { nodes, edges, cache } = parseBoard(
      board.data,
      executeBoard,
      openTraces,
      executeCommand,
      selected.current,
      currentRun,
      currentProfile.data?.flow_settings.connection_mode,
    );

    setNodes(nodes);
    setEdges(edges);
    setPinCache(new Map(cache));
    setBoardMeta({
      name: board.data.name,
      description: board.data.description,
      stage: board.data.stage,
      logLevel: board.data.log_level,
    });
  }, [board.data, currentRun]);

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
    () => ({ flowNode: FlowNode, commentNode: CommentNode }),
    [],
  );
  const { screenToFlowPosition, setViewport } = useReactFlow();

  async function executeCommands(commands: { command: string; args: any }[]) {
    let first = true;
    for (const { command, args } of commands) {
      await invoke(command, { ...args, append: !first });
      first = false;
    }
    if (commands.length > 0) {
      await board.refetch();
      console.log("Refetched board, execute commands");
    }
  }

  const onConnect = useCallback(
    (params: any) =>
      setEdges((eds) => {
        const [sourcePin, sourceNode] = pinCache.get(params.sourceHandle) || [];
        const [targetPin, targetNode] = pinCache.get(params.targetHandle) || [];

        if (!sourcePin || !targetPin) return eds;
        if (!sourceNode || !targetNode) return eds;

        executeCommand("connect_pins", {
          boardId: boardId,
          fromNode: sourceNode.id,
          fromPin: sourcePin.id,
          toNode: targetNode.id,
          toPin: targetPin.id,
        });

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
    [screenToFlowPosition, pinCache],
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
              let foundNode = Object.values(board.data?.nodes || {}).find(
                (node) => node.id === change.id,
              );
              if (foundNode)
                return {
                  command: "remove_node_from_board",
                  args: { boardId: boardId, node: foundNode },
                };
              let foundComment = Object.values(board.data?.comments || {}).find(
                (comment) => comment.id === change.id,
              );
              if (foundComment)
                return {
                  command: "remove_comment",
                  args: { boardId: boardId, comment: foundComment },
                };
              return undefined;
            })
            .filter((command) => command !== undefined) as any[],
        );

        return applyNodeChanges(changes, nds);
      }),
    [setNodes, board.data, boardId],
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
              let [fromPin, fromNode] = pinCache.get(fromPinId) || [];
              let [toPin, toNode] = pinCache.get(toPinId) || [];

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

      await executeCommand("disconnect_pins", {
        boardId,
        fromNode: oldEdge.source,
        fromPin: oldEdge.sourceHandle,
        toNode: oldEdge.target,
        toPin: oldEdge.targetHandle,
      });
      await executeCommand(
        "connect_pins",
        {
          boardId,
          fromNode: newConnection.source,
          fromPin: newConnection.sourceHandle,
          toNode: newConnection.target,
          toPin: newConnection.targetHandle,
        },
        true,
      );

      edgeReconnectSuccessful.current = true;
      setEdges((els) => reconnectEdge(oldEdge, newConnection, els));
    },
    [boardId, setEdges],
  );

  const onReconnectEnd = useCallback(
    async (event: any, edge: any) => {
      if (!edgeReconnectSuccessful.current) {
        const { source, target, sourceHandle, targetHandle } = edge;
        await executeCommand("disconnect_pins", {
          boardId,
          fromNode: source,
          fromPin: sourceHandle,
          toNode: target,
          toPin: targetHandle,
        });
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
      let first = true;
      for await (const node of nodes) {
        console.log(
          `Moving node ${node.id} to ${node.position.x}, ${node.position.y}`,
        );
        let comment = Object.values(board.data?.comments || {}).find(
          (comment) => comment.id === node.id,
        );
        if (comment)
          await invoke("upsert_comment", {
            boardId: boardId,
            comment: {
              ...comment,
              coordinates: [node.position.x, node.position.y, 0],
            },
            append: !first,
          });
        if (!comment)
          await invoke("move_node", {
            boardId: boardId,
            nodeId: node.id,
            coordinates: [node.position.x, node.position.y, 0],
            append: !first,
          });
        first = false;
      }
      await board.refetch();
    },
    [boardId],
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
            await invoke("update_board_meta", {
              boardId: boardId,
              name: boardMeta.name,
              description: boardMeta.description,
              logLevel: boardMeta.logLevel,
              stage: boardMeta.stage,
            });
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
            ...(currentRun
              ? [
                  {
                    icon: <ScrollIcon />,
                    title: "Logs",
                    onClick: async () => {
                      setTraces((old) =>
                        old
                          ? undefined
                          : { node: undefined, traces: currentRun.traces },
                      );
                    },
                  },
                ]
              : ([] as any)),
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
                  let new_comment: IComment = {
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
                  await executeCommand("upsert_comment", {
                    boardId: boardId,
                    comment: new_comment,
                  });
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
                    {openBoards.data?.map(([boardLoadId, boardName]) => (
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
                            await invoke("close_board", {
                              boardId: boardLoadId,
                            });
                            if (boardLoadId === boardId) {
                              const nextBoard = openBoards.data?.find(
                                ([id]) => id !== boardId,
                              );
                              if (nextBoard) {
                                await openBoards.refetch();
                                router.push(`/flow?id=${nextBoard[0]}`);
                                return;
                              }
                              router.push(`/`);
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
                    onlyRenderVisibleElements={false}
                    edges={edges}
                    onNodesChange={onNodesChangeIntercept}
                    onEdgesChange={onEdgesChange}
                    onNodeDragStop={onNodeDragStop}
                    isValidConnection={isValidConnectionCB}
                    onConnect={onConnect}
                    onReconnect={onReconnect}
                    onReconnectStart={onReconnectStart}
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
              hidden={!traces || !currentRun}
              ref={logPanelRef}
              collapsible={true}
              autoSave="flow-log"
            >
              {traces && currentRun && (
                <Traces
                  node={traces.node}
                  result={currentRun}
                  traces={traces.traces}
                  onOpenChange={(open) => {
                    if (!open) setTraces(undefined);
                  }}
                />
              )}
            </ResizablePanel>
          </ResizablePanelGroup>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
