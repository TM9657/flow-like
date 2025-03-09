import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
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
import type { INode } from "../../../lib/schema/flow/node";

export function FlowNodeRenameMenu({
	node,
	boardId,
	open,
	onOpenChange,
}: Readonly<{
	node: INode;
	boardId: string;
	open: boolean;
	onOpenChange: (open: boolean) => void;
}>) {
	const queryClient = useQueryClient();
	const [friendlyName, setFriendlyName] = useState(node.friendly_name);

	async function saveComment() {
		await invoke("update_node", {
			boardId: boardId,
			node: { ...node, friendly_name: friendlyName },
		});
		onOpenChange(false);
		setFriendlyName("");
		refetchBoard();
	}

	async function refetchBoard() {
		queryClient.invalidateQueries({
			queryKey: ["get", "board", boardId],
		});
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
