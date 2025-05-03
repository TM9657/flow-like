"use client";

import {
	type Node,
	type NodeProps,
	NodeResizer,
	type ResizeDragEvent,
	type ResizeParams,
	useReactFlow,
} from "@xyflow/react";
import { SquarePenIcon } from "lucide-react";
import { useCallback, useState } from "react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import type { IComment } from "../../lib/schema/flow/board";
import { Button } from "../ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";
import { MarkdownComponent } from "../ui/markdown";
import { Textarea } from "../ui/textarea";

export type CommentNode = Node<
	{
		comment: IComment;
		onUpsert: (comment: IComment) => Promise<void>;
		boardId: string;
		appId: string;
		hash: string;
	},
	"commentNode"
>;

export function CommentNode(props: NodeProps<CommentNode>) {
	const { getNodes } = useReactFlow();
	const [edit, setEdit] = useState({
		open: false,
		content: props.data.comment.content,
	});

	const onResizeEnd = useCallback(
		async (event: ResizeDragEvent, params: ResizeParams) => {
			const node = getNodes().find((n) => n.id === props.id);
			if (!node) return;
			const comment = node.data.comment as IComment;
			props.data.onUpsert({
				...comment,
				coordinates: [params.x, params.y, props.data.comment.coordinates[2]],
				width: params.width,
				height: params.height,
			});
		},
		[props.data.comment, props.data.onUpsert, props.width, props.height],
	);

	return (
		<>
			<NodeResizer
				color="#ff0071"
				handleStyle={{
					width: 10,
					height: 10,
					zIndex: 1,
				}}
				isVisible={props.selected}
				onResizeEnd={onResizeEnd}
				minWidth={200}
				minHeight={80}
			/>
			<ContextMenu>
				<ContextMenuTrigger>
					<div
						key={`${props.id}__node`}
						className={`bg-card p-1 react-flow__node-default selectable !w-full !h-full focus:ring-2 relative rounded-md group opacity-80 ${props.selected && "!border-primary border-2"}`}
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
						<div className="text-start">
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
		</>
	);
}
