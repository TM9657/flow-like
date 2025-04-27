"use client";

import type { Node, NodeProps } from "@xyflow/react";
import { FoldHorizontalIcon, MessageSquareIcon, SquarePenIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "../../components/ui/context-menu";
import { IPinType, type IComment, type ILayer } from "../../lib/schema/flow/board";
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
import type { INode } from "../../lib";
import { typeToColor } from "./utils";

export type LayerNode = Node<
	{
		layer: ILayer;
		pinLookup: Record<string, INode>;
		boardId: string;
		hash: string;
		appId: string;
	},
	"layerNode"
>;

export function LayerNode(props: NodeProps<LayerNode>) {
	const divRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const height = Math.max(
			Object.values(props.data.layer.pins).filter(pin => pin.pin_type === IPinType.Input).length,
			Object.values(props.data.layer.pins).filter(pin => pin.pin_type === IPinType.Output).length
		)



		if (divRef.current) {
			divRef.current.style.height = `calc(${height * 15}px + 1.25rem + 0.5rem)`;
			divRef.current.style.minHeight = `calc(15px + 1.25rem + 0.5rem)`;
		}
	}, [])

	return (
		<ContextMenu>
			<ContextMenuTrigger>
				<div
				ref={divRef}
					key={`${props.id}__node`}
					className={`p-1 flex flex-col justify-center items-center react-flow__node-default selectable focus:ring-2 relative !bg-card rounded-md group ${props.selected && "!border-primary border-2"}`}
				>
					<div className="header absolute top-0 left-0 right-0 h-4 gap-1 flex flex-row items-center border-b-1 border-b-foreground bg-muted p-1 justify-start rounded-t-md">
						<FoldHorizontalIcon className="w-2 h-2" />
						<small className="font-medium leading-none">{props.data.layer.name}</small>
					</div>
					{
						Object.values(props.data.layer.pins).filter(pin => pin.pin_type === IPinType.Input).toSorted((a, b) => a.index - b.index).map((pin, index) => <FlowPin
							appId={props.data.appId}
							node={props.data.pinLookup[pin.id]}
							boardId={props.data.boardId}
							index={index}
							pin={pin}
							key={pin.id}
							skipOffset={true}
							onPinRemove={async () => {}}
						/>)
					}
					{
						Object.values(props.data.layer.pins).filter(pin => pin.pin_type === IPinType.Output).toSorted((a, b) => a.index - b.index).map((pin, index) => <FlowPin
							appId={props.data.appId}
							node={props.data.pinLookup[pin.id]}
							boardId={props.data.boardId}
							index={index}
							pin={pin}
							key={pin.id}
							skipOffset={true}
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
