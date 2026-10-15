import { useTranslation } from "@flow-like/locales";
import { useRef, useState } from "react";
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
import { useUndoRedo } from "../flow-history";

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
	const { t } = useTranslation("flow");
	const { pushCommand } = useUndoRedo(appId, boardId);
	const invalidate = useInvalidateInvoke();
	const backend = useBackend();
	const [friendlyName, setFriendlyName] = useState(node.friendly_name);
	const baseName = useRef(node.friendly_name);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string>();

	async function saveName() {
		if (saving) return;
		setSaving(true);
		setError(undefined);
		try {
			const board = await backend.boardState.getBoard(appId, boardId);
			const current =
				board.nodes[node.id] ??
				Object.values(board.layers ?? {})
					.flatMap((layer) => Object.values(layer.nodes ?? {}))
					.find((candidate) => candidate.id === node.id);
			if (!current)
				throw new Error(
					t(
						"nodeRenameMissingNode",
						"This node no longer exists. Your draft is still open.",
					),
				);
			if (current.friendly_name !== baseName.current)
				throw new Error(
					t(
						"nodeRenameChanged",
						"This name changed while you were editing. Reopen it to load the latest name. Your draft is still open.",
					),
				);
			const result = await backend.boardState.executeCommand(
				appId,
				boardId,
				updateNodeCommand({
					node: { ...current, friendly_name: friendlyName },
				}),
			);
			await pushCommand(result);
			await refetchBoard();
			onOpenChange(false);
		} catch (cause) {
			setError(
				cause instanceof Error
					? cause.message
					: t(
							"nodeRenameFailed",
							"Could not rename the node. Your draft is still open.",
						),
			);
		} finally {
			setSaving(false);
		}
	}

	async function refetchBoard() {
		await invalidate(backend.boardState.getBoard, [appId, boardId]);
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
					<DialogTitle>{t("renameNode", "Rename Node")}</DialogTitle>
				</DialogHeader>
				<DialogDescription>
					{t("nodeRenameDescription", "Choose the name shown on this node.")}
				</DialogDescription>
				<Input
					aria-label={t("nodeName", "Node name")}
					value={friendlyName}
					onChange={(e) => setFriendlyName(e.target.value)}
				/>
				{error && (
					<p role="alert" className="text-sm text-destructive">
						{error}
					</p>
				)}
				<DialogFooter>
					<Button
						onClick={() => {
							onOpenChange(false);
						}}
						variant={"secondary"}
					>
						{t("cancel", "Cancel")}
					</Button>
					<Button disabled={saving} onClick={() => void saveName()}>
						{t("save", "Save")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
