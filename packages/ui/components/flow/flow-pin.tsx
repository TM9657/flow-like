import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuLabel,
    ContextMenuTrigger,
} from "../../components/ui/context-menu";
import { type INode } from "../../lib/schema/flow/node";
import { IValueType, type IPin } from "../../lib/schema/flow/pin";
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useDebounce } from "@uidotdev/usehooks";
import { Handle, Position, useInternalNode } from '@xyflow/react';
import { useCallback, useEffect, useState } from 'react';
import { DynamicImage } from '../ui/dynamic-image';
import { PinEdit } from "./flow-pin/pin-edit";
import { typeToColor } from './utils';
import { EllipsisIcon, EllipsisVerticalIcon, GripIcon, ListIcon } from "lucide-react";
import { IVariableType } from "../../lib/schema/flow/variable";

export function FlowPinInner({ pin, index, boardId, node }: Readonly<{ pin: IPin, index: number, boardId: string, node: INode }>) {
    const queryClient = useQueryClient()
    const currentNode = useInternalNode(node.id)

    const [defaultValue, setDefaultValue] = useState(pin.default_value);
    const debouncedDefaultValue = useDebounce(defaultValue, 200)

    const updateNode = useCallback(async () => {
        if (debouncedDefaultValue === undefined) return
        if (debouncedDefaultValue === null) return
        if (debouncedDefaultValue === pin.default_value) return
        if (!currentNode) return
        await invoke("update_node", { boardId: boardId, node: { ...node, coordinates: [currentNode.position.x, currentNode.position.y, 0], pins: { ...node.pins, [pin.id]: { ...pin, default_value: debouncedDefaultValue } } } })
        await refetchBoard()
    }, [debouncedDefaultValue, currentNode])

    useEffect(() => {
        updateNode()
    }, [debouncedDefaultValue])

    useEffect(() => {
        setDefaultValue(pin.default_value)
    }, [pin])

    async function refetchBoard() {
        queryClient.invalidateQueries({
            queryKey: ["get", "board", boardId]
        })
    }

    return <Handle
        type={pin.pin_type === "Input" ? "target" : "source"}
        position={pin.pin_type === "Input" ? Position.Left : Position.Right}
        id={pin.id}
        style={{
            marginTop: "1.75rem",
            top: index * 15,
            background: (pin.data_type === "Execution" || pin.value_type !== IValueType.Normal) ? "transparent" : typeToColor(pin.data_type),
        }}
        className='flex flex-row items-center gap-1'
    >
        {pin.data_type === "Execution" && <DynamicImage url="/flow/pin.svg" className='w-2 h-2 absolute left-0 -translate-x-[15%] pointer-events-none bg-foreground' />}
        {pin.value_type === IValueType.Array && <GripIcon strokeWidth={3} className={`w-2 h-2 absolute left-0 -translate-x-[30%] pointer-events-none bg-background`} style={{color: typeToColor(pin.data_type), backgroundColor: "var(--xy-node-background-color, var(--xy-node-background-color-default))"}}/>}
        {pin.value_type === IValueType.HashSet && <EllipsisVerticalIcon strokeWidth={3} className={`w-2 h-2 absolute left-0 -translate-x-[30%] pointer-events-none bg-background`} style={{color: typeToColor(pin.data_type), backgroundColor: "var(--xy-node-background-color, var(--xy-node-background-color-default))"}}/>}
        {pin.value_type === IValueType.HashMap && <ListIcon strokeWidth={3} className={`w-2 h-2 absolute left-0 -translate-x-[30%] pointer-events-none`} style={{color: typeToColor(pin.data_type), backgroundColor: "var(--xy-node-background-color, var(--xy-node-background-color-default))"}}/>}
        {(pin.name !== "exec_in" && pin.name !== "exec_out" && pin.name !== "var_ref") && <div className={`flex flex-row items-center gap-1 max-w-1/2 ${pin.pin_type === "Input" ? "ml-2" : "translate-x-[calc(-100%-0.25rem)]"}`}>
            <PinEdit pin={pin} defaultValue={defaultValue} changeDefaultValue={(value) => {setDefaultValue(value)}} />
        </div>}
    </Handle>
}

export function FlowPin({ pin, index, boardId, node, onPinRemove }: Readonly<{ pin: IPin, index: number, boardId: string, node: INode, onPinRemove: (pin: IPin) => Promise<void> }>) {

    if (pin.dynamic) return <ContextMenu>
        <ContextMenuTrigger>
            <FlowPinInner pin={pin} index={index} boardId={boardId} node={node} />
        </ContextMenuTrigger>
        <ContextMenuContent>
            <ContextMenuLabel>Pin Actions</ContextMenuLabel>
            <ContextMenuItem onClick={() => { onPinRemove(pin) }}>Remove Pin</ContextMenuItem>
        </ContextMenuContent>
    </ContextMenu>

    return FlowPinInner({ pin, index, boardId, node })
}