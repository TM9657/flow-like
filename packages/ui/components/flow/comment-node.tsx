"use client";

import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import { type IComment } from "../../lib/schema/flow/board";
import { type Node, type NodeProps } from "@xyflow/react";
import { MessageSquareIcon, SquarePenIcon } from "lucide-react";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";
import { useState } from "react";
import { Textarea } from "../ui/textarea";
import { Button } from "../ui/button";
import { MarkdownComponent } from "../ui/markdown";

export type CommentNode = Node<
	{
		comment: IComment;
		onUpsert: (comment: IComment) => Promise<void>;
		boardId: string;
	},
	"commentNode"
>;

export function CommentNode(props: NodeProps<CommentNode>) {
	const [edit, setEdit] = useState({
		open: false,
		content: props.data.comment.content,
	});

	return (
		<ContextMenu>
			<ContextMenuTrigger>
				<div
					key={props.id + "__node"}
					className={`bg-card p-1 react-flow__node-default selectable !w-[300px] focus:ring-2 relative rounded-md group opacity-80 ${props.selected && "!border-primary border-2"}`}
				>
					<Dialog
						open={edit.open}
						onOpenChange={(open) => {
							setEdit((old) => ({ ...old, open }));
						}}
					>
						<DialogContent>
							<DialogHeader>
								<DialogTitle>Edit Comment</DialogTitle>
								<DialogDescription>
									Edit the text content of the comment.
								</DialogDescription>
							</DialogHeader>
							<Textarea
								rows={6}
								value={edit.content}
								onChange={(e) => {
									setEdit({ ...edit, content: e.target.value });
								}}
							/>
							<Button
								disabled={props.data.comment.content === edit.content}
								onClick={async () => {
									await props.data.onUpsert({
										...props.data.comment,
										content: edit.content,
									});
									setEdit((old) => ({ ...old, open: false }));
								}}
							>
								Save
							</Button>
						</DialogContent>
					</Dialog>
					<div className="header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b-1 border-b-foreground bg-secondary p-1 justify-start rounded-md">
						<MessageSquareIcon className="w-2 h-2" />
						<small className="font-medium leading-none">Comment</small>
					</div>
					<div className="pt-4 text-start">
						<MarkdownComponent content={props.data.comment.content} />
					</div>
				</div>
			</ContextMenuTrigger>
			<ContextMenuContent className="max-w-20">
				<ContextMenuItem
					className="flex flex-row items-center gap-2"
					onClick={() => {
						setEdit((old) => ({ ...old, open: true }));
					}}
				>
					<SquarePenIcon className="w-4 h-4" />
					Edit
				</ContextMenuItem>
			</ContextMenuContent>
		</ContextMenu>
	);
}
