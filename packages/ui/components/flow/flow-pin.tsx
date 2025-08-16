"use client";
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
import type { ILayer } from "../../lib/schema/flow/board";
import type { INode } from "../../lib/schema/flow/node";
import { type IPin, IValueType } from "../../lib/schema/flow/pin";
import { useBackendStore } from "../../state/backend-state";
import { DynamicImage } from "../ui/dynamic-image";
import { useUndoRedo } from "./flow-history";
import { PinEdit } from "./flow-pin/pin-edit";
import { typeToColor } from "./utils";

function FlowPinInnerComponent({
	pin,
	boardId,
	appId,
	node,
	skipOffset,
}: Readonly<{
	pin: IPin;
	boardId: string;
	appId: string;
	node: INode | ILayer;
	skipOffset?: boolean;
}>) {
	const { pushCommand } = useUndoRedo(appId, boardId);
	const invalidate = useInvalidateInvoke();
	const currentNode = useInternalNode(node?.id);

	const [defaultValue, setDefaultValue] = useState(pin.default_value);

	const handleStyle = useMemo(() => {
		if (node?.name === "reroute") {
			return {
				// marginTop: "0.5rem",
				// top: index * 15,
				background: typeToColor(pin.data_type),
			};
		}

		if (skipOffset) {
			return {
				marginTop: "1.75rem",
				top: (pin.index - 1) * 15,
				background:
					pin.data_type === "Execution" || pin.value_type !== IValueType.Normal
						? "transparent"
						: typeToColor(pin.data_type),
			};
		}

		return {
			marginTop: "1.75rem",
			top: (pin.index - 1) * 15,
			background:
				pin.data_type === "Execution" || pin.value_type !== IValueType.Normal
					? "transparent"
					: typeToColor(pin.data_type),
		};
	}, [pin.data_type, pin.value_type, pin.index, node?.name]);

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
			node?.name !== "reroute",
		[pin.name, node?.name],
	);

	const pinEditContainerClassName = useMemo(
		() =>
			`flex flex-row items-center gap-1 max-w-1/2 ${pin.pin_type === "Input" ? "ml-2" : "translate-x-[calc(-100%-0.25rem)]"}`,
		[pin.pin_type],
	);

	const refetchBoard = useCallback(async () => {
		const backend = useBackendStore.getState().backend;
		if (!backend) return;
		invalidate(backend.boardState.getBoard, [appId, boardId]);
	}, [appId, boardId, invalidate]);

	const updateNode = useCallback(async () => {
		if (node.nodes) return;
		const translatedNode = node as INode;
		if (defaultValue === undefined) return;
		if (defaultValue === null) return;
		if (defaultValue === pin.default_value) return;
		if (!currentNode) return;
		const backend = useBackendStore.getState().backend;
		if (!backend) return;
		const command = updateNodeCommand({
			node: {
				...translatedNode,
				hash: undefined,
				coordinates: [currentNode.position.x, currentNode.position.y, 0],
				pins: {
					...node.pins,
					[pin.id]: { ...pin, default_value: defaultValue },
				},
			},
		});

		const result = await backend.boardState.executeCommand(
			currentNode.data.appId as string,
			boardId,
			command,
		);
		await pushCommand(result, false);
		await refetchBoard();
	}, [
		pin.id,
		defaultValue,
		currentNode,
		refetchBoard,
		boardId,
		node,
		pushCommand,
	]);

	useEffect(() => {
		updateNode();
	}, [defaultValue]);

	useEffect(() => {
		setDefaultValue(pin.default_value);
	}, [pin]);

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
				{pin.data_type === "Execution" && node?.name !== "reroute" && (
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
		[pin.data_type, pin.value_type, iconStyle, node?.name],
	);

	return (
		<MemoizedHandle
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
						nodeId={node.id}
						pin={pin}
						appId={appId}
						boardId={boardId}
						defaultValue={defaultValue}
						changeDefaultValue={setDefaultValue}
					/>
				</div>
			)}
		</MemoizedHandle>
	);
}

const MemoizedHandle = memo(Handle);

function pinPropsAreEqual(prevProps: any, nextProps: any) {
	return (
		// prevProps.index === nextProps.index &&
		prevProps.boardId === nextProps.boardId &&
		prevProps.node?.id === nextProps.node?.id &&
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
	boardId,
	appId,
	node,
	onPinRemove,
	skipOffset,
}: Readonly<{
	pin: IPin;
	boardId: string;
	appId: string;
	node: INode | ILayer;
	skipOffset?: boolean;
	onPinRemove: (pin: IPin) => Promise<void>;
}>) {
	if (pin.dynamic)
		return (
			<ContextMenu>
				<ContextMenuTrigger>
					<FlowPinInner
						appId={appId}
						pin={pin}
						boardId={boardId}
						node={node}
						skipOffset={skipOffset}
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
			boardId={boardId}
			node={node}
			skipOffset={skipOffset}
		/>
	);
}

const pin = memo(FlowPin);
export { pin as FlowPin };
