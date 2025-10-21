"use client";

import { useInternalNode } from "@xyflow/react";
import { isEqual } from "lodash-es";
import { useCallback, useEffect, useState } from "react";
import { useInvalidateInvoke } from "../../../hooks";
import { type INode, updateNodeCommand } from "../../../lib";
import { useBackend } from "../../../state/backend-state";
import useFlowControlState from "../../../state/flow-control-state";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "../../ui";
import { useUndoRedo } from "../flow-history";
import { VariablesMenuEdit } from "../variables/variables-menu-edit";

export function PinEditModal({
	appId,
	boardId,
	version,
}: Readonly<{ appId: string; boardId: string; version?: [number, number, number] }>) {
	const backend = useBackend();
	const invalidate = useInvalidateInvoke();
	const { pushCommand } = useUndoRedo(appId, boardId);
	const [defaultValueState, setDefaultValueState] = useState<any>(null);
	const { editedPin, stopEditPin } = useFlowControlState();
	const currentNode = useInternalNode(editedPin?.node ?? "");

	useEffect(() => {
		if (editedPin) {
			setDefaultValueState(editedPin.pin.default_value);
			return;
		}

		setDefaultValueState(null);
	}, [editedPin?.node, editedPin?.pin]);

	const refetchBoard = useCallback(async () => {
		invalidate(backend.boardState.getBoard, [appId, boardId]);
	}, [appId, boardId, backend, invalidate]);

	const onChangeDefaultValue = useCallback(async () => {
		if (typeof version !== "undefined") {
			stopEditPin();
			return;
		}

		if (!editedPin?.node || !currentNode || !editedPin?.pin) {
			stopEditPin();
			return;
		}

		const hasChanged = !isEqual(defaultValueState, editedPin.pin.default_value);
		if (!hasChanged) {
			stopEditPin();
			return;
		}

		const node = currentNode.data.node as INode;

		if (!node) {
			stopEditPin();
			return;
		}

		const command = updateNodeCommand({
			node: {
				...node,
				coordinates: [currentNode.position.x, currentNode.position.y, 0],
				pins: {
					...node.pins,
					[editedPin.pin.id]: {
						...editedPin.pin,
						default_value: defaultValueState,
					},
				},
			},
		});

		const result = await backend.boardState.executeCommand(
			appId,
			boardId,
			command,
		);
		await pushCommand(result, false);
		await refetchBoard();
		stopEditPin();
	}, [
		appId,
		currentNode,
		editedPin?.node,
		editedPin?.pin,
		defaultValueState,
		refetchBoard,
		pushCommand,
		backend,
		boardId,
		stopEditPin,
		version,
	]);

	if (!editedPin?.pin || !currentNode) {
		return null;
	}

	return (
		<Dialog
			open={!!editedPin.pin && !!currentNode}
			onOpenChange={async (open) => {
				if (!open) {
					await onChangeDefaultValue();
				}
			}}
		>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Set Default Value</DialogTitle>
					<DialogDescription>
						The default value will only be used if the pin is not connected.
					</DialogDescription>
				</DialogHeader>
				<div className="w-full">
					<VariablesMenuEdit
						variable={{
							data_type: editedPin.pin.data_type,
							default_value: defaultValueState,
							exposed: false,
							id: editedPin.pin.id,
							value_type: editedPin.pin.value_type,
							name: editedPin.pin.name,
							editable: editedPin.pin.editable,
							secret: editedPin.pin.secret,
							category: editedPin.pin.category,
							description: editedPin.pin.description,
						}}
						updateVariable={async (variable) => {
							setDefaultValueState(variable.default_value);
						}}
					/>
				</div>
			</DialogContent>
		</Dialog>
	);
}
