"use client"

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger
} from "../../components/ui/context-menu";
import { toastSuccess } from '../../lib/messages';
import {useRunExecutionStore} from "../../state/run-execution-state";
import { createId } from "@paralleldrive/cuid2";
import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useDebounce } from "@uidotdev/usehooks";
import { type Node, type NodeProps, useNodes } from '@xyflow/react';
import { AlignCenterVerticalIcon, AlignEndVerticalIcon, AlignHorizontalSpaceAroundIcon, AlignStartVerticalIcon, AlignVerticalJustifyCenterIcon, AlignVerticalJustifyEndIcon, AlignVerticalJustifyStartIcon, AlignVerticalSpaceAroundIcon, ClockIcon, CopyIcon, MessageSquareIcon, PlayCircleIcon, ScrollTextIcon, SquareCheckIcon, SquarePenIcon, WorkflowIcon } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';
import { DynamicImage } from '../ui/dynamic-image';
import { FlowNodeCommentMenu } from './flow-node/flow-node-comment-menu';
import { FlowPinAction } from "./flow-node/flow-node-pin-action";
import { FlowNodeRenameMenu } from "./flow-node/flow-node-rename-menu";
import { FlowPin } from './flow-pin';
import PuffLoader from "react-spinners/PuffLoader";
import { useTheme } from "next-themes";
import { type INode } from "../../lib/schema/flow/node";
import { type ITrace } from "../../lib/schema/flow/run";
import { type IPin } from "../../lib/schema/flow/pin";
import { type IComment } from "../../lib/schema/flow/board";

export interface IPinAction {
  action: "create",
  pin: IPin,
  onAction: (pin: IPin) => Promise<void>
}

export type FlowNode = Node<
  {
    node: INode;
    boardId: string;
    traces: ITrace[];
    onExecute: (node: INode) => Promise<void>;
    openTrace: (trace: ITrace[]) => Promise<void>;
  },
  'node'
>;

export function FlowNode(props: NodeProps<FlowNode>) {
  const { resolvedTheme } = useTheme()
  const queryClient = useQueryClient()
  const [executing, setExecuting] = useState(false);
  const [isExec, setIsExec] = useState(false)
  const [commentMenu, setCommentMenu] = useState(false)
  const [renameMenu, setRenameMenu] = useState(false)
  const [inputPins, setInputPins] = useState<(IPin | IPinAction)[]>([])
  const [outputPins, setOutputPins] = useState<(IPin | IPinAction)[]>([])
  const { runs } = useRunExecutionStore()
  const [executionState, setExecutionState] = useState<"done" | "running" | "none">("none")
  const debouncedExecutionState = useDebounce(executionState, 100);
  const div = useRef<HTMLDivElement>(null)
  const nodes = useNodes();
  const scope = useMemo(() => {
    const selected = nodes.filter(node => node.selected)
    const self = selected.find(node => node.id === props.id)
    if (!self) {
      return [...selected, nodes.filter(node => node.id === props.id)[0]]
    }

    return selected
  }, [nodes])

  function sortPins(a: IPin, b: IPin) {
    // Step 1: Compare by type - Input comes before Output
    if (a.pin_type === "Input" && b.pin_type === "Output") return -1;
    if (a.pin_type === "Output" && b.pin_type === "Input") return 1;

    // Step 2: If types are the same, compare by index
    return a.index - b.index;
  }

  useEffect(() => {
    const height = Math.max(inputPins.length, outputPins.length)
    div.current!.style.height = `calc(${height * 15}px + 1.25rem + 0.5rem)`

  }, [inputPins, outputPins])

  useEffect(() => {
    parsePins(Object.values(props.data.node?.pins || []))
  }, [props.data.node.pins, props.positionAbsoluteX, props.positionAbsoluteY])

  useEffect(() => {
    let isRunning = false;
    let already_executed = false;

    for (const [_, run] of runs) {
      if (run.nodes.has(props.id)) {
        isRunning = true;
        break;
      }

      if (run.already_executed.has(props.id)) {
        already_executed = true;
      }
    }

    if(isRunning) {
      setExecutionState("running");
      return;
    }

    if(already_executed) {
      setExecutionState("done");
      return;
    }

    setExecutionState("none");
  }, [runs, props.id]);

  const addPin = useCallback(async (node: INode, pin: IPin, index: number) => {
    const nodeGuard = nodes.find(node => node.id === props.id)
    if (!nodeGuard) return

    node = nodeGuard.data.node as INode;
    if (!node.pins) return
    const newPin: IPin = {
      ...pin,
      depends_on: [],
      connected_to: [],
      id: createId(),
      index: index,
    }

    let pins = Object.values(node.pins).sort(sortPins)
    pins.splice(index, 0, newPin)
    node.pins = {}
    pins.forEach((pin, index) => node.pins[pin.id] = { ...pin, index: index })
    await invoke("update_node", { boardId: props.data.boardId, node: { ...node, coordinates: [nodeGuard.position.x, nodeGuard.position.y, 0] }, append: false })
    queryClient.invalidateQueries({
      queryKey: ["get", "board", props.data.boardId]
    })
  }, [nodes])

  const pinRemoveCallback = useCallback(async (pin: IPin) => {
    const nodeGuard = nodes.find(node => node.id === props.id)
    if (!nodeGuard) return

    if (!props.data.node.pins) return
    const node = nodes.find(node => node.id === props.id)?.data.node as INode;
    const pins = Object.values(node.pins).filter(p => p.id !== pin.id).sort(sortPins)
    node.pins = {}
    pins.forEach((pin, index) => props.data.node.pins[pin.id] = { ...pin, index: index })
    await invoke("update_node", { boardId: props.data.boardId, node: { ...node, coordinates: [nodeGuard.position.x, nodeGuard.position.y, 0] }, append: false })
    queryClient.invalidateQueries({
      queryKey: ["get", "board", props.data.boardId]
    })
  }, [inputPins, outputPins, nodes])

  function parsePins(pins: IPin[]) {
    const inputPins: (IPin | IPinAction)[] = []
    const outputPins: (IPin | IPinAction)[] = []
    let isExec = false

    let pastPinWithCount: [string, number, IPin | undefined] = ["", 0, undefined]

    Object.values(pins).sort(sortPins).forEach((pin, index) => {
      if (pin.data_type === "Execution") isExec = true

      let pastPinId = pin.name + "_" + pin.pin_type

      if (pastPinWithCount[0] === pastPinId) {
        pastPinWithCount[1] += 1
      }

      if (pastPinWithCount[0] !== pastPinId && pastPinWithCount[1] > 0) {
        const action: IPinAction = {
          action: "create",
          pin: { ...pastPinWithCount[2]! },
          onAction: async (pin) => {
            await addPin(props.data.node, pin, index - 1)
          }
        }

        if (pastPinWithCount[2]?.pin_type === "Input") {
          inputPins.push(action)
        } else {
          outputPins.push(action)
        }
      }

      // update to past pin information
      if (pastPinWithCount[0] !== pastPinId) pastPinWithCount = [pastPinId, 0, pin]
      pin = { ...pin, dynamic: pastPinWithCount[1] > 1 }

      if (pin.pin_type === "Input") {
        inputPins.push(pin)
      } else {
        outputPins.push(pin)
      }
    })

    if (pastPinWithCount[1] > 0 && pastPinWithCount[2]) {
      const action: IPinAction = {
        action: "create",
        pin: { ...pastPinWithCount[2] },
        onAction: async (pin) => {
          await addPin(props.data.node, pin, Object.values(props.data.node?.pins || []).length)
        }
      }

      if (pastPinWithCount[2].pin_type === "Input") {
        inputPins.push(action)
      } else {
        outputPins.push(action)
      }
    }

    setInputPins(inputPins)
    setOutputPins(outputPins)
    setIsExec(isExec)
  }

  const copy = useCallback(async () => {
    const selectedNodes: INode[] = scope.filter((node: any) => node.selected && node.type === "flowNode").map((node: any) => node.data.node)
    const selectedComments: IComment[] = scope.filter((node: any) => node.selected && node.type === "commentNode").map((node: any) => node.data.comment)
    try {
      navigator.clipboard.writeText(JSON.stringify({ nodes: selectedNodes, comments: selectedComments }, null, 2))
      toastSuccess("Nodes copied to clipboard", <CopyIcon className="w-4 h-4" />)
      return;
    } catch (error) {
      toast.error("Failed to copy nodes to clipboard")
    }
  }, [scope])

  function isPinAction(pin: IPin | IPinAction): pin is IPinAction {
    return typeof (pin as IPinAction).onAction === "function"
  }

  return (
    <ContextMenu key={props.id}>
      <ContextMenuTrigger asChild>
        <div key={props.id + "__node"} ref={div} className={`bg-card p-2 react-flow__node-default selectable focus:ring-2 relative rounded-md group ${props.selected && "!border-primary border-2"} ${isExec ? "" : "bg-emerald-900"} ${executionState === "done" ? "opacity-60" : "opacity-100"}`}>
          <FlowNodeCommentMenu boardId={props.data.boardId} node={props.data.node} open={commentMenu} onOpenChange={(open) => setCommentMenu(open)} />
          <FlowNodeRenameMenu boardId={props.data.boardId} node={props.data.node} open={renameMenu} onOpenChange={(open) => setRenameMenu(open)} />
          {props.data.node.long_running && <div className='absolute top-0 z-10 translate-y-[calc(-50%)] translate-x-[calc(-50%)] left-0 text-center bg-background rounded-full'>
            <ClockIcon className='w-2 h-2 text-foreground' />
          </div>}
          {props.data.node.comment && <div className='absolute top-0 translate-y-[calc(-100%-0.5rem)] left-3 right-3 mb-2 text-center bg-foreground/70 text-background p-1 rounded-md'>
            <small className='font-normal text-extra-small leading-extra-small'>{props.data.node.comment}</small>
          </div>}
          {props.data.node.error && <div className='absolute bottom-0 translate-y-[calc(100%+1rem)] left-3 right-3 mb-2 text-destructive-foreground bg-destructive p-1 rounded-md'>
            <small className='font-normal text-extra-small leading-extra-small'>{props.data.node.error}</small>
          </div>}
          {inputPins.filter((pin) => isPinAction(pin) || pin.name !== "var_ref").map((pin, index) => isPinAction(pin) ?
            <FlowPinAction key={pin.pin.id + "__action"} action={pin} index={index} input /> :
            <FlowPin key={pin.id} node={props.data.node} boardId={props.data.boardId} index={index} pin={pin} onPinRemove={pinRemoveCallback} />
          )}
          <div className={`header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b-1 border-b-foreground p-1 justify-between rounded-md rounded-b-none bg-card ${!isExec && "bg-gradient-to-r  from-card via-emerald-300/50 to-emerald-300 dark:via-tertiary/50 dark:to-tertiary"} ${props.data.node.start && "bg-gradient-to-r  from-card via-rose-300/50 to-rose-300 dark:via-primary/50 dark:to-primary"}`}>
            <div className={`flex flex-row items-center gap-1`}>
              {props.data.node?.icon && <DynamicImage className='w-2 h-2 bg-foreground' url={props.data.node?.icon} />}
              {!props.data.node?.icon && <WorkflowIcon className='w-2 h-2' />}
              <small className='font-medium leading-none text-start line-clamp-1'>{props.data.node?.friendly_name}</small>
            </div>
            <div className="flex flex-row items-center gap-1">
              {props.data.traces.length > 0 && <ScrollTextIcon onClick={() => props.data.openTrace(props.data.traces)} className="w-2 h-2 cursor-pointer hover:text-primary" />}
              {props.data.node.start && !executing && <PlayCircleIcon className="w-2 h-2 cursor-pointer hover:text-primary" onClick={async (e) => {
                if (executing) return
                setExecuting(true)
                await props.data.onExecute(props.data.node)
                setExecuting(false)
              }} />}
              {debouncedExecutionState === "running" && <PuffLoader color={resolvedTheme === "dark" ? "white" : "black"} size={10} speedMultiplier={1} />}
              {debouncedExecutionState === "done" && <SquareCheckIcon className="w-2 h-2 text-primary" />}
            </div>
          </div>
          {outputPins.map((pin, index) => isPinAction(pin) ?
            <FlowPinAction action={pin} index={index} input={false} key={pin.pin.id + "__action"} /> :
            <FlowPin node={props.data.node} boardId={props.data.boardId} index={index} pin={pin} key={pin.id} onPinRemove={pinRemoveCallback} />
          )}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent className='max-w-20'>
        <ContextMenuLabel>Node Actions</ContextMenuLabel>
        {scope.length <= 1 && props.data.node.start && <ContextMenuItem onClick={() => setRenameMenu(true)}>
          <div className='flex flex-row items-center gap-2 text-nowrap'>
            <SquarePenIcon className='w-4 h-4' />
            Rename
          </div>
        </ContextMenuItem>}
        {scope.length <= 1 && <ContextMenuItem onClick={() => setCommentMenu(true)}>
          <div className='flex flex-row items-center gap-2 text-nowrap'>
            <MessageSquareIcon className='w-4 h-4' />
            Comment
          </div>
        </ContextMenuItem>}
        <ContextMenuItem onClick={async () => await copy()}>
          <div className='flex flex-row items-center gap-2 text-nowrap'>
            <CopyIcon className='w-4 h-4' />
            Copy
          </div>
        </ContextMenuItem>
        {scope.length > 1 && <>
          <ContextMenuSeparator />
          <ContextMenuSub>
            <ContextMenuSubTrigger>
              <div className='flex flex-row items-center gap-2 text-nowrap'>
                <AlignStartVerticalIcon className='w-4 h-4' />
                Align
              </div>
            </ContextMenuSubTrigger>
            <ContextMenuSubContent>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignStartVerticalIcon className='w-4 h-4' />
                  Start
                </div>
              </ContextMenuItem>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignCenterVerticalIcon className='w-4 h-4' />
                  Center
                </div>
              </ContextMenuItem>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignEndVerticalIcon className='w-4 h-4' />
                  End
                </div>
              </ContextMenuItem>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignHorizontalSpaceAroundIcon className='w-4 h-4' />
                  Space Around
                </div>
              </ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuSub>

          <ContextMenuSeparator />
          <ContextMenuSub>
            <ContextMenuSubTrigger>
              <div className='flex flex-row items-center gap-2 text-nowrap'>
                <AlignVerticalJustifyStartIcon className='w-4 h-4' />
                Justify
              </div>
            </ContextMenuSubTrigger>
            <ContextMenuSubContent>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignVerticalJustifyStartIcon className='w-4 h-4' />
                  Start
                </div>
              </ContextMenuItem>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignVerticalJustifyCenterIcon className='w-4 h-4' />
                  Center
                </div>
              </ContextMenuItem>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignVerticalJustifyEndIcon className='w-4 h-4' />
                  End
                </div>
              </ContextMenuItem>
              <ContextMenuItem>
                <div className='flex flex-row items-center gap-2 text-nowrap'>
                  <AlignVerticalSpaceAroundIcon className='w-4 h-4' />
                  Space Around
                </div>
              </ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuSub>
        </>}
      </ContextMenuContent>
    </ContextMenu>
  );
}