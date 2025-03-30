import { useState } from "react";
import { Button } from "../../../components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../../components/ui/dialog";
import { Input } from "../../../components/ui/input";
import { useInvalidateInvoke } from "../../../hooks";
import { updateNodeCommand } from "../../../lib";
import type { INode } from "../../../lib/schema/flow/node";
import { useBackend } from "../../../state/backend-state";

export function FlowNodeRenameMenu({
	node,
	boardId,
	appId,
	open,
	onOpenChange,
}: Readonly<{
	node: INode;
	appId: string;
	boardId: string;
	open: boolean;
	onOpenChange: (open: boolean) => void;
}>) {
	const invalidate = useInvalidateInvoke();
	const backend = useBackend();
	const [friendlyName, setFriendlyName] = useState(node.friendly_name);

	async function saveComment() {
		const command = updateNodeCommand({
			node: { ...node, friendly_name: friendlyName },
		});

		await backend.executeCommand(appId, boardId, command, false);
		onOpenChange(false);
		setFriendlyName("");
		refetchBoard();
	}

	async function refetchBoard() {
		await invalidate(backend.getBoard, [appId, boardId]);
	}

	return (
		<Dialog
			open={open}
			onOpenChange={(open) => {
				onOpenChange(open);
			}}
		>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Rename Node</DialogTitle>
				</DialogHeader>
				<DialogDescription>
					<Input
						value={friendlyName}
						onChange={(e) => {
							setFriendlyName(e.target.value);
						}}
					/>
				</DialogDescription>
				<DialogFooter>
					<Button
						onClick={() => {
							onOpenChange(false);
						}}
						variant={"secondary"}
					>
						Cancel
					</Button>
					<Button onClick={async () => await saveComment()}>Save</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
