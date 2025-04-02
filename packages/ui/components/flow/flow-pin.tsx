"use client";
import { useDebounce } from "@uidotdev/usehooks";
import {
	Handle,
	type HandleType,
	Position,
	useInternalNode,
} from "@xyflow/react";
import { EllipsisVerticalIcon, GripIcon, ListIcon } from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useState } from "react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuLabel,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import { useInvalidateInvoke } from "../../hooks";
import { updateNodeCommand } from "../../lib";
import type { INode } from "../../lib/schema/flow/node";
import { type IPin, IValueType } from "../../lib/schema/flow/pin";
import { useBackend } from "../../state/backend-state";
import { DynamicImage } from "../ui/dynamic-image";
import { useUndoRedo } from "./flow-history";
import { PinEdit } from "./flow-pin/pin-edit";
import { typeToColor } from "./utils";

function FlowPinInnerComponent({
	pin,
	index,
	boardId,
	appId,
	node,
}: Readonly<{
	pin: IPin;
	index: number;
	boardId: string;
	appId: string;
	node: INode;
}>) {
	const {pushCommand} = useUndoRedo(appId, boardId);
 	const invalidate = useInvalidateInvoke();
	const backend = useBackend();
	const currentNode = useInternalNode(node.id);

	const [defaultValue, setDefaultValue] = useState(pin.default_value);
	const debouncedDefaultValue = useDebounce(defaultValue, 200);

	const handleStyle = useMemo(
		() => ({
			marginTop: "1.75rem",
			top: index * 15,
			background:
				pin.data_type === "Execution" || pin.value_type !== IValueType.Normal
					? "transparent"
					: typeToColor(pin.data_type),
		}),
		[pin.data_type, pin.value_type, index],
	);

	const iconStyle = useMemo(
		() => ({
			color: typeToColor(pin.data_type),
			backgroundColor:
				"var(--xy-node-background-color, var(--xy-node-background-color-default))",
		}),
		[pin.data_type],
	);

	const shouldRenderPinEdit = useMemo(
		() =>
			pin.name !== "exec_in" &&
			pin.name !== "exec_out" &&
			pin.name !== "var_ref",
		[pin.name],
	);

	const pinEditContainerClassName = useMemo(
		() =>
			`flex flex-row items-center gap-1 max-w-1/2 ${pin.pin_type === "Input" ? "ml-2" : "translate-x-[calc(-100%-0.25rem)]"}`,
		[pin.pin_type],
	);

	const updateNode = useCallback(async () => {
		if (debouncedDefaultValue === undefined) return;
		if (debouncedDefaultValue === null) return;
		if (debouncedDefaultValue === pin.default_value) return;
		if (!currentNode) return;
		const command = updateNodeCommand({
			node: {
				...node,
				coordinates: [currentNode.position.x, currentNode.position.y, 0],
				pins: {
					...node.pins,
					[pin.id]: { ...pin, default_value: debouncedDefaultValue },
				},
			},
		});

		const result = await backend.executeCommand(
			currentNode.data.appId as string,
			boardId,
			command,
		);
		await pushCommand(result, false)
		await refetchBoard();
	}, [pin.id, debouncedDefaultValue, currentNode]);

	useEffect(() => {
		updateNode();
	}, [debouncedDefaultValue]);

	useEffect(() => {
		setDefaultValue(pin.default_value);
	}, [pin]);

	const refetchBoard = useCallback(async () => {
		invalidate(backend.getBoard, [appId, boardId]);
	}, [appId, boardId]);

	const pinTypeProps = useMemo(
		() => ({
			type: pin.pin_type === "Input" ? "target" : "source",
			position: pin.pin_type === "Input" ? Position.Left : Position.Right,
		}),
		[pin.pin_type],
	);

	// Memoize the pin icons rendering based on type
	const pinIcons = useMemo(
		() => (
			<>
				{pin.data_type === "Execution" && (
					<DynamicImage
						url="/flow/pin.svg"
						className="w-2 h-2 absolute left-0 -translate-x-[15%] pointer-events-none bg-foreground"
					/>
				)}
				{pin.value_type === IValueType.Array && (
					<GripIcon
						strokeWidth={3}
						className="w-2 h-2 absolute left-0 -translate-x-[30%] pointer-events-none bg-background"
						style={iconStyle}
					/>
				)}
				{pin.value_type === IValueType.HashSet && (
					<EllipsisVerticalIcon
						strokeWidth={3}
						className="w-2 h-2 absolute left-0 -translate-x-[30%] pointer-events-none bg-background"
						style={iconStyle}
					/>
				)}
				{pin.value_type === IValueType.HashMap && (
					<ListIcon
						strokeWidth={3}
						className="w-2 h-2 absolute left-0 -translate-x-[30%] pointer-events-none"
						style={iconStyle}
					/>
				)}
			</>
		),
		[pin.data_type, pin.value_type, iconStyle],
	);

	return (
		<Handle
			type={pinTypeProps.type as HandleType}
			position={pinTypeProps.position}
			id={pin.id}
			style={handleStyle}
			className="flex flex-row items-center gap-1"
		>
			{pinIcons}
			{shouldRenderPinEdit && (
				<div className={pinEditContainerClassName}>
					<PinEdit
						pin={pin}
						defaultValue={defaultValue}
						changeDefaultValue={setDefaultValue}
					/>
				</div>
			)}
		</Handle>
	);
}

function pinPropsAreEqual(prevProps: any, nextProps: any) {
	return (
		prevProps.index === nextProps.index &&
		prevProps.boardId === nextProps.boardId &&
		prevProps.node.id === nextProps.node.id &&
		prevProps.pin.id === nextProps.pin.id &&
		prevProps.pin.default_value === nextProps.pin.default_value &&
		prevProps.pin.data_type === nextProps.pin.data_type &&
		prevProps.pin.value_type === nextProps.pin.value_type &&
		prevProps.pin.pin_type === nextProps.pin.pin_type
	);
}

export const FlowPinInner = memo(FlowPinInnerComponent, pinPropsAreEqual);
function FlowPin({
	pin,
	index,
	boardId,
	appId,
	node,
	onPinRemove,
}: Readonly<{
	pin: IPin;
	index: number;
	boardId: string;
	appId: string;
	node: INode;
	onPinRemove: (pin: IPin) => Promise<void>;
}>) {
	if (pin.dynamic)
		return (
			<ContextMenu>
				<ContextMenuTrigger>
					<FlowPinInner
						appId={appId}
						pin={pin}
						index={index}
						boardId={boardId}
						node={node}
					/>
				</ContextMenuTrigger>
				<ContextMenuContent>
					<ContextMenuLabel>Pin Actions</ContextMenuLabel>
					<ContextMenuItem
						onClick={() => {
							onPinRemove(pin);
						}}
					>
						Remove Pin
					</ContextMenuItem>
				</ContextMenuContent>
			</ContextMenu>
		);

	return (
		<FlowPinInner
			appId={appId}
			pin={pin}
			index={index}
			boardId={boardId}
			node={node}
		/>
	);
}

const pin = memo(FlowPin);
export { pin as FlowPin };
