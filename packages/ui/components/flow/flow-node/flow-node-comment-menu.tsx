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
import { Textarea } from "../../../components/ui/textarea";
import { useInvalidateInvoke } from "../../../hooks";
import { updateNodeCommand } from "../../../lib";
import type { INode } from "../../../lib/schema/flow/node";
import { useBackend } from "../../../state/backend-state";
import { useUndoRedo } from "../flow-history";

export function FlowNodeCommentMenu({
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
	const [comment, setComment] = useState(node.comment ?? "");
	const baseComment = useRef(node.comment ?? "");
	const saveInFlight = useRef(false);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string>();

	async function saveComment() {
		if (saveInFlight.current) return;
		if (comment === baseComment.current) {
			onOpenChange(false);
			return;
		}
		saveInFlight.current = true;
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
						"nodeCommentMissingNode",
						"This node no longer exists. Your draft is still open.",
					),
				);
			if ((current.comment ?? "") !== baseComment.current) {
				throw new Error(
					t(
						"nodeCommentChanged",
						"This comment changed while you were editing. Reopen it to load the latest text. Your draft is still open.",
					),
				);
			}
			const result = await backend.boardState.executeCommand(
				appId,
				boardId,
				updateNodeCommand({ node: { ...current, comment } }),
			);
			await pushCommand(result);
			await refetchBoard();
			onOpenChange(false);
		} catch (cause) {
			setError(
				cause instanceof Error
					? cause.message
					: t(
							"nodeCommentSaveFailed",
							"Could not save the comment. Your draft is still open.",
						),
			);
		} finally {
			saveInFlight.current = false;
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
				if (open) onOpenChange(true);
				else void saveComment();
			}}
		>
			<DialogContent aria-busy={saving}>
				<DialogHeader>
					<DialogTitle>{t("comment", "Comment")}</DialogTitle>
				</DialogHeader>
				<DialogDescription>
					{t("nodeCommentDescription", "Show a note above this node.")}
				</DialogDescription>
				<Textarea
					aria-label={t("nodeComment", "Node comment")}
					rows={6}
					value={comment}
					disabled={saving}
					onChange={(e) => {
						if (!saveInFlight.current) setComment(e.target.value);
					}}
				/>
				{error && (
					<p role="alert" className="text-sm text-destructive">
						{error}
					</p>
				)}
				<DialogFooter>
					<Button
						disabled={saving}
						onClick={() => {
							if (!saveInFlight.current) onOpenChange(false);
						}}
						variant={"secondary"}
					>
						{t("cancel", "Cancel")}
					</Button>
					<Button disabled={saving} onClick={() => void saveComment()}>
						{t("save", "Save")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
