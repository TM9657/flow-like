"use client";

import {
	type Node,
	type NodeProps,
	NodeResizer,
	type ResizeDragEvent,
	type ResizeParams,
	useReactFlow,
} from "@xyflow/react";
import {
	LockIcon,
	SquareChevronDownIcon,
	SquareChevronUpIcon,
	SquarePenIcon,
	UnlockIcon,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import type { IComment } from "../../lib/schema/flow/board";
import { Button, TextEditor } from "../ui";
import { ColorPicker } from "../ui/color-picker";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";

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
	const { getNodes, setNodes } = useReactFlow();
	const [edit, setEdit] = useState({
		open: false,
		content: props.data.comment.content,
	});
	const [colorPickerOpen, setColorPickerOpen] = useState(false);
	const [currentColor, setCurrentColor] = useState<string | undefined>(
		props.data.comment.color ?? undefined,
	);

	const isLocked = props.data.comment.is_locked ?? false;

	const toggleLock = useCallback(async () => {
		const next = !isLocked;
		// Try to persist on the comment if supported
		const node = getNodes().find((n) => n.id === props.id);
		if (node) {
			const comment = node.data.comment as IComment;
			try {
				console.dir({
					...comment,
					is_locked: next,
				});
				await props.data.onUpsert({
					...comment,
					is_locked: next,
				});
			} catch {
				// noop
			}
		}
	}, [getNodes, isLocked, props.data.onUpsert, props.id, setNodes]);

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

	const onColorChosen = useCallback(async () => {
		const node = getNodes().find((n) => n.id === props.id);
		if (!node) return;
		const comment = node.data.comment as IComment;
		props.data.onUpsert({
			...comment,
			color: currentColor,
		});
	}, [
		props.data.comment,
		props.data.onUpsert,
		props.width,
		props.height,
		currentColor,
	]);

	const onMoveLayer = useCallback(
		async (by: number) => {
			const node = getNodes().find((n) => n.id === props.id);
			if (!node) return;
			const comment = node.data.comment as IComment;
			props.data.onUpsert({
				...comment,
				z_index: (props.data.comment.z_index ?? 1) + by,
			});
		},
		[
			props.data.comment,
			props.data.onUpsert,
			props.width,
			props.height,
			currentColor,
		],
	);

	return (
		<>
			<NodeResizer
				color="#ff0071"
				handleStyle={{
					width: 10,
					height: 10,
					zIndex: (props.data.comment.z_index ?? 1) + 1,
				}}
				isVisible={!isLocked && props.selected}
				onResizeEnd={onResizeEnd}
				minWidth={30}
				minHeight={30}
			/>
			<ContextMenu>
				<ContextMenuTrigger>
					<div
						key={`${props.id}__node`}
						className={`bg-card p-1 md-wrapper react-flow__node-default w-full! h-full! focus:ring-2 relative rounded-md! border-0! group opacity-80 ${props.selected && ""} ${isLocked ? "cursor-not-allowed" : ""}`}
						style={{
							backgroundColor: currentColor,
						}}
					>
						<div className="absolute top-1 right-1 z-50 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
							<Button
								variant="secondary"
								size="icon"
								title={isLocked ? "Unlock comment" : "Lock comment"}
								onClick={(e) => {
									e.preventDefault();
									e.stopPropagation();
									toggleLock();
								}}
								className="h-6 w-6"
							>
								{isLocked ? (
									<LockIcon className="w-3.5 h-3.5" />
								) : (
									<UnlockIcon className="w-3.5 h-3.5" />
								)}
							</Button>
						</div>
						{props.selected && (
							<ColorPicker
								className="z-50 absolute top-0 left-0 border translate-x-[-120%] "
								value={currentColor ?? "#ffffff"}
								onChange={(value) => setCurrentColor(value)}
								open={colorPickerOpen}
								onOpenChange={(open) => {
									setColorPickerOpen(open);
									if (!open) {
										onColorChosen();
									}
								}}
							/>
						)}
						<Dialog
							open={edit.open}
							onOpenChange={async (open) => {
								if (!open) {
									await props.data.onUpsert({
										...props.data.comment,
										content: edit.content,
									});
									toast.success("Comment updated successfully");
								}
								setEdit((old) => ({ ...old, open }));
							}}
						>
							<DialogContent className="max-w-(--breakpoint-xl) min-w-[95dvw] w-full min-h-[90vh] max-h-[90vh] overflow-hidden flex flex-col">
								<DialogHeader>
									<DialogTitle>Edit Comment</DialogTitle>
									<DialogDescription>
										Edit the text content of the comment.
									</DialogDescription>
								</DialogHeader>
								<div className="flex flex-col grow max-h-full overflow-auto relative">
									<TextEditor
										initialContent={
											props.data.comment.content === ""
												? "Empty Comment"
												: props.data.comment.content
										}
										onChange={(content) => {
											setEdit((old) => ({ ...old, content }));
										}}
										isMarkdown={true}
										editable={true}
									/>
								</div>
							</DialogContent>
						</Dialog>
						<div className="text-start relative">
							<TextEditor
								initialContent={
									props.data.comment.content === ""
										? "Empty Comment"
										: props.data.comment.content
								}
								isMarkdown={true}
								editable={false}
							/>
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
					<ContextMenuSeparator />
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							onMoveLayer(1);
						}}
					>
						<SquareChevronUpIcon className="w-4 h-4" />
						Move Up
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							onMoveLayer(-1);
						}}
					>
						<SquareChevronDownIcon className="w-4 h-4" />
						Move Down
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							toggleLock();
						}}
					>
						{isLocked ? (
							<UnlockIcon className="size-4" />
						) : (
							<LockIcon className="size-4" />
						)}
						{isLocked ? "Unlock comment" : "Lock comment"}
					</ContextMenuItem>
				</ContextMenuContent>
			</ContextMenu>
		</>
	);
}
