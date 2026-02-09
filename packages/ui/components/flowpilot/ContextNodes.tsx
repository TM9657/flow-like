"use client";

import { ChevronDownIcon, FocusIcon, LayersIcon } from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";
import type { IBoard } from "../../lib/schema/flow/board";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../ui/collapsible";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

interface ContextNodesProps {
	nodeIds: string[];
	board: IBoard | null | undefined;
	onSelectNodes?: (nodeIds: string[]) => void;
	onFocusNode?: (nodeId: string) => void;
	compact?: boolean;
}

const MAX_VISIBLE_NODES = 3;

export const ContextNodes = memo(function ContextNodes({
	nodeIds,
	board,
	onSelectNodes,
	onFocusNode,
	compact = false,
}: ContextNodesProps) {
	const [isExpanded, setIsExpanded] = useState(false);

	const nodeDetails = useMemo(() => {
		if (!board?.nodes || nodeIds.length === 0) return [];
		return nodeIds
			.map((id) => {
				const node = board.nodes[id];
				if (!node) return null;
				return {
					id,
					name: node.friendly_name || node.name,
				};
			})
			.filter(Boolean) as { id: string; name: string }[];
	}, [nodeIds, board?.nodes]);

	const handleReselectAll = useCallback(() => {
		if (onSelectNodes && nodeIds.length > 0) {
			onSelectNodes(nodeIds);
		}
	}, [onSelectNodes, nodeIds]);

	if (nodeDetails.length === 0) return null;

	const shouldCollapse = nodeDetails.length > MAX_VISIBLE_NODES;

	if (compact) {
		if (shouldCollapse) {
			return (
				<Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
					<div className="flex flex-wrap items-center gap-1 my-1.5">
						<CollapsibleTrigger asChild>
							<button
								type="button"
								className="flex items-center gap-1 px-1.5 py-0.5 text-[10px] text-primary-foreground bg-linear-to-r from-primary/90 via-violet-500/90 to-pink-500/90 hover:from-primary hover:via-violet-500 hover:to-pink-500 rounded-full transition-all duration-200 shadow-sm hover:shadow-md"
							>
								<LayersIcon className="w-2.5 h-2.5" />
								<span className="font-medium">{nodeDetails.length} nodes</span>
								<ChevronDownIcon
									className={`w-2.5 h-2.5 transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
								/>
							</button>
						</CollapsibleTrigger>
						{!isExpanded && (
							<Tooltip>
								<TooltipTrigger asChild>
									<button
										type="button"
										onClick={handleReselectAll}
										className="text-[10px] text-muted-foreground hover:text-foreground transition-colors underline-offset-2 hover:underline"
									>
										re-select all
									</button>
								</TooltipTrigger>
								<TooltipContent side="top" className="text-xs">
									Select all {nodeDetails.length} nodes on canvas
								</TooltipContent>
							</Tooltip>
						)}
					</div>
					<CollapsibleContent>
						<div className="flex flex-wrap items-center gap-1 mt-1 pl-1 border-l-2 border-primary/30">
							<Tooltip>
								<TooltipTrigger asChild>
									<button
										type="button"
										onClick={handleReselectAll}
										className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors mr-1"
									>
										<FocusIcon className="w-2.5 h-2.5" />
									</button>
								</TooltipTrigger>
								<TooltipContent side="top" className="text-xs">
									Re-select all nodes
								</TooltipContent>
							</Tooltip>
							{nodeDetails.map(({ id, name }) => (
								<Badge
									key={id}
									variant="outline"
									className="text-[10px] px-1 py-0 h-4 cursor-pointer hover:bg-accent/80 transition-colors font-normal"
									onClick={() => onFocusNode?.(id)}
								>
									{name}
								</Badge>
							))}
						</div>
					</CollapsibleContent>
				</Collapsible>
			);
		}

		return (
			<div className="flex flex-wrap items-center gap-1 my-1.5">
				<Tooltip>
					<TooltipTrigger asChild>
						<button
							type="button"
							onClick={handleReselectAll}
							className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
						>
							<FocusIcon className="w-2.5 h-2.5" />
							<span>Context:</span>
						</button>
					</TooltipTrigger>
					<TooltipContent side="top" className="text-xs">
						Click to re-select these nodes
					</TooltipContent>
				</Tooltip>
				{nodeDetails.map(({ id, name }) => (
					<Badge
						key={id}
						variant="outline"
						className="text-[10px] px-1 py-0 h-4 cursor-pointer hover:bg-accent/80 transition-colors font-normal"
						onClick={() => onFocusNode?.(id)}
					>
						{name}
					</Badge>
				))}
			</div>
		);
	}

	// Non-compact mode (input area indicator)
	if (shouldCollapse) {
		return (
			<Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
				<div className="flex items-center gap-2 mb-2">
					<CollapsibleTrigger asChild>
						<button
							type="button"
							className="flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium text-primary-foreground bg-linear-to-r from-primary via-violet-500 to-pink-500 hover:from-primary/90 hover:via-violet-500/90 hover:to-pink-500/90 rounded-lg transition-all duration-200 shadow-sm hover:shadow-md"
						>
							<LayersIcon className="w-3.5 h-3.5" />
							<span>{nodeDetails.length} nodes selected</span>
							<ChevronDownIcon
								className={`w-3.5 h-3.5 transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
							/>
						</button>
					</CollapsibleTrigger>
					{!isExpanded && (
						<Tooltip>
							<TooltipTrigger asChild>
								<Button
									variant="ghost"
									size="sm"
									className="h-6 px-2 text-xs"
									onClick={handleReselectAll}
								>
									<FocusIcon className="w-3 h-3 mr-1" />
									Re-select
								</Button>
							</TooltipTrigger>
							<TooltipContent side="top" className="text-xs">
								Select all {nodeDetails.length} nodes on canvas
							</TooltipContent>
						</Tooltip>
					)}
				</div>
				<CollapsibleContent>
					<div className="flex flex-wrap items-center gap-1.5 mb-2 pl-2 border-l-2 border-primary/40">
						{nodeDetails.map(({ id, name }) => (
							<Badge
								key={id}
								variant="secondary"
								className="text-[10px] px-1.5 py-0 h-5 cursor-pointer hover:bg-accent/80 transition-colors"
								onClick={() => onFocusNode?.(id)}
							>
								{name}
							</Badge>
						))}
					</div>
				</CollapsibleContent>
			</Collapsible>
		);
	}

	// Non-compact, few nodes
	return (
		<div className="flex flex-wrap items-center gap-2 mb-2">
			<Tooltip>
				<TooltipTrigger asChild>
					<button
						type="button"
						onClick={handleReselectAll}
						className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
					>
						<FocusIcon className="w-3.5 h-3.5" />
						<span>Context:</span>
					</button>
				</TooltipTrigger>
				<TooltipContent side="top" className="text-xs">
					Click to re-select these nodes
				</TooltipContent>
			</Tooltip>
			{nodeDetails.map(({ id, name }) => (
				<Badge
					key={id}
					variant="secondary"
					className="text-xs px-2 py-0.5 h-6 cursor-pointer hover:bg-accent/80 transition-colors"
					onClick={() => onFocusNode?.(id)}
				>
					{name}
				</Badge>
			))}
		</div>
	);
});
