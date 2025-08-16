"use client";
import { type Node, type NodeProps, useReactFlow } from "@xyflow/react";
import {
	FoldHorizontalIcon,
	MessageSquareIcon,
	SlidersHorizontalIcon,
	SquarePenIcon,
	Trash2Icon,
	ZapIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuLabel,
	ContextMenuSeparator,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import type { INode } from "../../lib";
import { type ILayer, IPinType } from "../../lib/schema/flow/board";
import { CommentDialog } from "./comment-dialog";
import { FlowPin } from "./flow-pin";
import { LayerEditMenu } from "./layer-editing-menu";
import { NameDialog } from "./name-dialog";

export type LayerNode = Node<
	{
		layer: ILayer;
		pinLookup: Record<string, INode>;
		boardId: string;
		hash: string;
		appId: string;
		pushLayer(layer: ILayer): Promise<void>;
		onLayerUpdate(layer: ILayer): Promise<void>;
		onLayerRemove(layer: ILayer, preserve_nodes: boolean): Promise<void>;
	},
	"layerNode"
>;

export function LayerNode(props: NodeProps<LayerNode>) {
	const divRef = useRef<HTMLDivElement>(null);
	const { getNodes } = useReactFlow();
	const [comment, setComment] = useState<string | undefined>();
	const [name, setName] = useState<string | undefined>();
	const [editing, setEditing] = useState(false);

	useEffect(() => {
		const height = Math.max(
			Object.values(props.data.layer.pins).filter(
				(pin) => pin.pin_type === IPinType.Input,
			).length,
			Object.values(props.data.layer.pins).filter(
				(pin) => pin.pin_type === IPinType.Output,
			).length,
		);

		if (divRef.current) {
			divRef.current.style.height = `calc(${height * 15}px + 1.25rem + 0.5rem)`;
			divRef.current.style.minHeight = "calc(15px + 1.25rem + 0.5rem)";
		}
	}, [props.data.hash]);

	const saveComment = useCallback(async () => {
		const node = getNodes().find((n) => n.id === props.id);
		if (!node) return;
		const layer = node.data.layer as ILayer;
		props.data.onLayerUpdate({ ...layer, comment: comment ?? "" });
		setComment(undefined);
	}, [props.id, comment]);

	const saveName = useCallback(async () => {
		const node = getNodes().find((n) => n.id === props.id);
		if (!node) return;
		const layer = node.data.layer as ILayer;
		props.data.onLayerUpdate({ ...layer, name: name ?? "Collapsed" });
		setName(undefined);
	}, [props.id, name]);

	return (
		<>
			{typeof comment === "string" && (
				<CommentDialog
					onOpenChange={(open) => {
						if (!open) {
							saveComment();
						}
					}}
					comment={comment}
					open={typeof comment === "string"}
					onUpsert={(comment) => setComment(comment)}
				/>
			)}
			{typeof name === "string" && (
				<NameDialog
					onOpenChange={(open) => {
						if (!open) {
							saveName();
						}
					}}
					name={name}
					open={typeof name === "string"}
					onUpsert={(name) => setName(name)}
				/>
			)}
			<ContextMenu>
				<ContextMenuTrigger>
					<div
						ref={divRef}
						key={`${props.data.hash}__node`}
						className={`p-1 flex flex-col justify-center items-center react-flow__node-default selectable focus:ring-2 relative bg-card! border-border! rounded-md! group ${props.selected && "border-primary! border-2"}`}
					>
						{props.data.layer.comment && props.data.layer.comment !== "" && (
							<div className="absolute top-0 translate-y-[calc(-100%-0.5rem)] left-3 right-3 mb-2 text-center bg-foreground/70 text-background p-1 rounded-md">
								<small className="font-normal text-extra-small leading-extra-small">
									{props.data.layer.comment}
								</small>
								<div
									className="
                                            absolute
                                            -bottom-1
                                            left-1/2
                                            transform -translate-x-1/2
                                            w-0 h-0
                                            border-l-4 border-l-transparent
                                            border-r-4 border-r-transparent
                                            border-t-4 border-t-foreground/70
                                        "
								/>
							</div>
						)}
						<div className="header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b bg-muted p-1 justify-start rounded-t-md">
							<ZapIcon className="w-2 h-2" />
							<small className="font-medium leading-none">
								{props.data.layer.name}
							</small>
						</div>
						{Object.values(props.data.layer.pins)
							.filter((pin) => pin.pin_type === IPinType.Input)
							.toSorted((a, b) => a.index - b.index)
							.map((pin) => (
								<FlowPin
									appId={props.data.appId}
									node={props.data.pinLookup[pin.id] ?? props.data.layer}
									boardId={props.data.boardId}
									pin={pin}
									key={pin.id}
									skipOffset={true}
									onPinRemove={async () => {}}
								/>
							))}
						{Object.values(props.data.layer.pins)
							.filter((pin) => pin.pin_type === IPinType.Output)
							.toSorted((a, b) => a.index - b.index)
							.map((pin) => (
								<FlowPin
									appId={props.data.appId}
									node={props.data.pinLookup[pin.id] ?? props.data.layer}
									boardId={props.data.boardId}
									pin={pin}
									key={pin.id}
									skipOffset={true}
									onPinRemove={async () => {}}
								/>
							))}
					</div>
				</ContextMenuTrigger>
				<ContextMenuContent className="max-w-20">
					<ContextMenuLabel>Layer Actions</ContextMenuLabel>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							setName(props.data.layer.name ?? "");
						}}
					>
						<SquarePenIcon className="w-4 h-4" />
						Rename
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => {
							setComment(props.data.layer.comment ?? "");
						}}
					>
						<MessageSquareIcon className="w-4 h-4" />
						Comment
					</ContextMenuItem>
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={() => setEditing(true)}
					>
						<SlidersHorizontalIcon className="w-4 h-4" />
						Edit
					</ContextMenuItem>
					<ContextMenuSeparator />
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={async () => {
							await props.data.onLayerRemove(props.data.layer, true);
						}}
					>
						<FoldHorizontalIcon className="w-4 h-4" />
						Extend
					</ContextMenuItem>
					<ContextMenuSeparator />
					<ContextMenuItem
						className="flex flex-row items-center gap-2"
						onClick={async () => {
							await props.data.onLayerRemove(props.data.layer, false);
						}}
					>
						<Trash2Icon className="w-4 h-4" />
						Delete
					</ContextMenuItem>
				</ContextMenuContent>
			</ContextMenu>

			<LayerEditMenu
				open={editing}
				layer={props.data.layer}
				onOpenChange={setEditing}
				onApply={async (updated) => {
					const newLayer = {
						...props.data.layer,
						pins: updated.pins,
					};
					try {
						await props.data.onLayerUpdate(newLayer);
					} catch (error) {
						console.error(error);
						toast.error("Failed to update layer");
					}
					setEditing(false);
				}}
			/>
		</>
	);
}
