"use client";

import type { Node, NodeProps } from "@xyflow/react";
import { MessageSquareIcon, SquarePenIcon } from "lucide-react";
import { useState } from "react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import type { IComment, ILayer } from "../../lib/schema/flow/board";
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
import { FlowPin } from "./flow-pin";

export type LayerNode = Node<
	{
		layer: ILayer;
		boardId: string;
		hash: string;
		appId: string;
	},
	"layerNode"
>;

export function LayerNode(props: NodeProps<LayerNode>) {
	// const [edit, setEdit] = useState({
	// 	open: false,
	// 	content: props.data.comment.content,
	// });

	return (
		<ContextMenu>
			<ContextMenuTrigger>
				<div
					key={`${props.id}__node`}
					className={`bg-card p-1 react-flow__node-default selectable focus:ring-2 relative rounded-md group opacity-80 ${props.selected && "!border-primary border-2"}`}
				>
					<div className="text-center">
						<p>{props.data.layer.name}</p>
					</div>
					{
						Object.values(props.data.layer.pins).map(pin => <FlowPin
							appId={props.data.appId}
							node={props.data as any}
							boardId={props.data.boardId}
							index={pin.index}
							pin={pin}
							key={pin.id}
							onPinRemove={async () => {}}
						/>)
					}
				</div>
			</ContextMenuTrigger>
			<ContextMenuContent className="max-w-20">
				<ContextMenuItem
					className="flex flex-row items-center gap-2"
					onClick={() => {
						// setEdit((old) => ({ ...old, open: true }));
					}}
				>
					<SquarePenIcon className="w-4 h-4" />
					Edit
				</ContextMenuItem>
			</ContextMenuContent>
		</ContextMenu>
	);
}
