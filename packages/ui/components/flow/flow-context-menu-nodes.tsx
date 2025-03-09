"use client";
import { ChevronRightIcon, WorkflowIcon } from "lucide-react";
import { useMemo } from "react";
import {
	ContextMenuItem,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
} from "../../components/ui/context-menu";
import type { INode } from "../../lib/schema/flow/node";
import type { IPin } from "../../lib/schema/flow/pin";
import { DynamicImage } from "../ui/dynamic-image";

export function FlowContextMenuNodes({
	items,
	filter,
	pin,
	onNodePlace,
}: Readonly<{
	items: INode[];
	filter: string;
	pin?: IPin;
	onNodePlace: (node: INode) => Promise<void>;
}>) {
	const nodeState = useMemo(() => {
		const leafs: INode[] = [];
		const nodes = new Map<string, INode[]>();

		items.forEach((item) => {
			const itemCopy = { ...item };
			const category = itemCopy.category.trim().split("/");

			if (category.length === 0 || category[0] === "") {
				leafs.push(itemCopy);
				return;
			}

			const root = category.shift() as string;
			itemCopy.category = category.join("/");

			if (!nodes.has(root)) {
				nodes.set(root, []);
			}
			nodes.get(root)?.push(itemCopy);
		});

		return { leafs, nodes };
	}, [items]);

	if (filter !== "") {
		return (
			<>
				{items.map((node) => (
					<ContextMenuItem
						key={node.id}
						id={node.id}
						onClick={() => onNodePlace(node)}
					>
						{node.icon ? (
							<DynamicImage
								url={node.icon}
								className="h-4 w-4 mr-2 bg-foreground"
							/>
						) : (
							<WorkflowIcon className="h-4 w-4 mr-2" />
						)}
						{node.friendly_name}
					</ContextMenuItem>
				))}
			</>
		);
	}

	return (
		<>
			{Array.from(nodeState.nodes)
				.sort(([categoryA], [categoryB]) => categoryA.localeCompare(categoryB))
				.map(([category, node]) => (
					<ContextMenuSub key={category + node.length}>
						<ContextMenuSubTrigger>
							<ChevronRightIcon className="h-4 w-4 mr-1" />
							{category}
						</ContextMenuSubTrigger>
						<ContextMenuSubContent className="w-48" key={category}>
							<FlowContextMenuNodes
								items={node}
								filter={filter}
								pin={pin}
								onNodePlace={onNodePlace}
							/>
						</ContextMenuSubContent>
					</ContextMenuSub>
				))}
			{nodeState.leafs.map((node) => (
				<ContextMenuItem
					key={"context" + node.id}
					id={node.id}
					onClick={async () => onNodePlace(node)}
				>
					{node.icon ? (
						<DynamicImage
							url={node.icon}
							className="min-h-4 min-w-4 mr-2 bg-foreground"
						/>
					) : (
						<WorkflowIcon className="h-4 w-4 mr-2" />
					)}
					{node.friendly_name}
				</ContextMenuItem>
			))}
		</>
	);
}
