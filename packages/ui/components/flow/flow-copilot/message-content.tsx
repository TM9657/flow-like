"use client";

import { BrainCircuitIcon, ChevronDown } from "lucide-react";
import { memo, useCallback, useMemo } from "react";

import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../../ui/collapsible";
import { TextEditor } from "../../ui/text-editor";

import type { IBoard } from "../../../lib";

interface MessageContentProps {
	content: string;
	onFocusNode?: (nodeId: string) => void;
	board?: IBoard;
}

export const MessageContent = memo(function MessageContent({
	content,
	onFocusNode,
	board,
}: MessageContentProps) {
	// Memoize getNodeName callback - returns display name for a valid node ID
	const getNodeName = useCallback(
		(nodeId: string): string | null => {
			if (!board?.nodes) return null;
			const node = board.nodes[nodeId];
			if (!node) return null;
			return node.friendly_name || node.node_type?.split("::").pop() || "Node";
		},
		[board?.nodes],
	);

	// Memoize preprocessing function - ONLY accepts valid node IDs
	const preprocessFocusNodes = useCallback(
		(text: string) => {
			let processedText = text;

			// Format 1: XML attribute style <focus_node node_id="..." ...>content</focus_node>
			const xmlAttrRegex =
				/<focus_node\s+node_id=["']([^"']+)["'][^>]*>[\s\S]*?<\/focus_node>/g;
			processedText = processedText.replace(
				xmlAttrRegex,
				(_match: string, nodeId: string) => {
					const trimmedId = nodeId.trim();
					if (!trimmedId) return "";

					const nodeName = getNodeName(trimmedId);
					if (nodeName) {
						return `[${nodeName}](focus://${trimmedId})`;
					}
					return `[${trimmedId}](invalid://node)`;
				},
			);

			// Format 2: Simple style <focus_node>nodeId</focus_node>
			const simpleTagRegex = /<focus_node>([\s\S]*?)<\/focus_node>/g;
			processedText = processedText.replace(
				simpleTagRegex,
				(_match: string, nodeContent: string) => {
					const nodeId = nodeContent.trim();
					if (!nodeId) return "";

					const nodeName = getNodeName(nodeId);
					if (nodeName) {
						return `[${nodeName}](focus://${nodeId})`;
					}
					return `[${nodeId}](invalid://node)`;
				},
			);

			// Clean up any unclosed/incomplete focus_node tags (during streaming)
			processedText = processedText
				.replace(/<focus_node[^>]*>([^<]*?)$/g, "") // Unclosed tag with attributes at end
				.replace(/<focus_node[^>]*>$/g, "") // Just opening tag at end
				.replace(/<focus_node>$/g, "") // Simple opening tag at end
				.replace(/<focus_n[^>]*$/g, "") // Partial opening tag
				.replace(/<\/focus_node>/g, "") // Orphan closing tags
				.replace(/<focus_node[^>]*>\s*<focus_node/g, "<focus_node"); // Nested opening tags

			return processedText;
		},
		[getNodeName],
	);

	// Memoize processed content
	const { thinkingMatch, openThinkingMatch, processedContent } = useMemo(() => {
		const thinkMatch = content.match(/<think>([\s\S]*?)<\/think>/);
		const openThinkMatch = content.match(/<think>([\s\S]*?)$/);
		const processed = preprocessFocusNodes(content);
		return {
			thinkingMatch: thinkMatch,
			openThinkingMatch: thinkMatch ? null : openThinkMatch,
			processedContent: processed,
		};
	}, [content, preprocessFocusNodes]);

	if (thinkingMatch) {
		const thinkingContent = thinkingMatch[1];
		const restContent = preprocessFocusNodes(
			content.replace(/<think>[\s\S]*?<\/think>/, "").trim(),
		);

		return (
			<div className="space-y-2 w-full">
				<Collapsible className="w-full border rounded-lg bg-background/50 overflow-hidden">
					<CollapsibleTrigger className="flex items-center gap-2 p-2 w-full hover:bg-muted/50 transition-colors text-xs font-medium text-muted-foreground group">
						<BrainCircuitIcon className="w-3 h-3" />
						<span>Reasoning Process</span>
						<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180" />
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="p-3 pt-0 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/30">
							{thinkingContent.trim()}
						</div>
					</CollapsibleContent>
				</Collapsible>
				<div className="text-sm leading-relaxed whitespace-break-spaces text-wrap max-w-full w-full">
					<TextEditor
						initialContent={restContent}
						isMarkdown={true}
						editable={false}
						onFocusNode={onFocusNode}
					/>
				</div>
			</div>
		);
	}

	if (openThinkingMatch) {
		const thinkingContent = openThinkingMatch[1];
		const beforeContent = preprocessFocusNodes(
			content.substring(0, openThinkingMatch.index).trim(),
		);

		return (
			<div className="space-y-2 w-full">
				{beforeContent && (
					<div className="text-sm leading-relaxed whitespace-break-spaces text-wrap max-w-full w-full">
						<TextEditor
							initialContent={beforeContent}
							isMarkdown={true}
							editable={false}
						/>
					</div>
				)}
				<Collapsible
					className="w-full border rounded-lg bg-background/50 overflow-hidden"
					defaultOpen={true}
				>
					<CollapsibleTrigger className="flex items-center gap-2 p-2 w-full hover:bg-muted/50 transition-colors text-xs font-medium text-muted-foreground group">
						<BrainCircuitIcon className="w-3 h-3 animate-pulse" />
						<span>Reasoning Process...</span>
						<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180" />
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="p-3 pt-0 text-xs text-muted-foreground whitespace-pre-wrap font-mono bg-muted/30">
							{thinkingContent.trim()}
							<span className="inline-block w-1.5 h-3 ml-1 bg-primary/50 animate-pulse" />
						</div>
					</CollapsibleContent>
				</Collapsible>
			</div>
		);
	}

	return (
		<div className="text-sm leading-relaxed whitespace-break-spaces text-wrap max-w-full w-full">
			<TextEditor
				initialContent={processedContent}
				isMarkdown={true}
				editable={false}
				onFocusNode={onFocusNode}
			/>
		</div>
	);
});
