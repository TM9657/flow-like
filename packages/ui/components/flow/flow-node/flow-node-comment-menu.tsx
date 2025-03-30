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
import { Textarea } from "../../../components/ui/textarea";
import type { INode } from "../../../lib/schema/flow/node";
import { updateNodeCommand } from "../../../lib";
import { useBackend } from "../../../state/backend-state";
import { useInvalidateInvoke } from "../../../hooks";

export function FlowNodeCommentMenu({
	node,
	boardId,
	appId,
	open,
	onOpenChange,
}: Readonly<{
	node: INode;
	appId: string,
	boardId: string;
	open: boolean;
	onOpenChange: (open: boolean) => void;
}>) {
	const invalidate = useInvalidateInvoke();
	const backend = useBackend()
	const [comment, setComment] = useState("");

	async function saveComment() {
		const command = updateNodeCommand({
			node: { ...node, comment }
		})

		await backend.executeCommand(appId, boardId, command, false)
		onOpenChange(false);
		setComment("");
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
					<DialogTitle>Comment</DialogTitle>
				</DialogHeader>
				<DialogDescription>
					<Textarea
						rows={6}
						value={comment}
						onChange={(e) => {
							setComment(e.target.value);
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
