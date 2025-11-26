"use client";

import { BrainCircuitIcon, ChevronDown } from "lucide-react";

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

export function MessageContent({
	content,
	onFocusNode,
	board,
}: MessageContentProps) {
	// Get node name from board by ID
	const getNodeName = (nodeId: string): string => {
		if (!board?.nodes) return "Node";
		const node = board.nodes[nodeId];
		return node?.friendly_name || node?.node_type?.split("::").pop() || "Node";
	};

	// Helper to resolve node ID from name, type, or ID
	const resolveNode = (
		identifier: string,
	): { id: string; name: string } | null => {
		if (!board?.nodes) return null;

		const trimmed = identifier.trim();
		const trimmedLower = trimmed.toLowerCase();

		// 1. Check if identifier is a valid node ID
		if (board.nodes[trimmed]) {
			return {
				id: trimmed,
				name: getNodeName(trimmed),
			};
		}

		// 2. Search by friendly_name (case-insensitive exact match)
		const nodeByFriendlyName = Object.values(board.nodes).find(
			(n) => n.friendly_name?.toLowerCase() === trimmedLower,
		);
		if (nodeByFriendlyName) {
			return {
				id: nodeByFriendlyName.id,
				name: nodeByFriendlyName.friendly_name || "Node",
			};
		}

		// 3. Search by node_type (exact match on last segment)
		const nodeByType = Object.values(board.nodes).find(
			(n) => n.node_type?.split("::").pop()?.toLowerCase() === trimmedLower,
		);
		if (nodeByType) {
			return {
				id: nodeByType.id,
				name:
					nodeByType.friendly_name ||
					nodeByType.node_type?.split("::").pop() ||
					"Node",
			};
		}

		// 4. Search by partial friendly_name match
		const nodeByPartialName = Object.values(board.nodes).find((n) =>
			n.friendly_name?.toLowerCase().includes(trimmedLower),
		);
		if (nodeByPartialName) {
			return {
				id: nodeByPartialName.id,
				name: nodeByPartialName.friendly_name || "Node",
			};
		}

		// 5. Search by partial node_type match
		const nodeByPartialType = Object.values(board.nodes).find((n) =>
			n.node_type?.toLowerCase().includes(trimmedLower),
		);
		if (nodeByPartialType) {
			return {
				id: nodeByPartialType.id,
				name:
					nodeByPartialType.friendly_name ||
					nodeByPartialType.node_type?.split("::").pop() ||
					"Node",
			};
		}

		return null;
	};

	// Preprocess content to convert focus tags to markdown links
	const preprocessFocusNodes = (text: string) => {
		// Match <focus_node>nodeId</focus_node> format
		const xmlRegex = /<focus_node>([^<]+)<\/focus_node>/g;

		// Convert to markdown links with node name from board
		const processedText = text.replace(
			xmlRegex,
			(_match: string, nodeContent: string) => {
				const trimmedContent = nodeContent.trim();
				const resolved = resolveNode(trimmedContent);

				if (resolved) {
					return `[${resolved.name}](focus://${resolved.id})`;
				}

				// Fallback: if it looks like a cuid2 (lowercase alphanumeric, 24+ chars), assume it's an ID
				if (/^[a-z0-9]{24,}$/.test(trimmedContent)) {
					return `[${getNodeName(trimmedContent)}](focus://${trimmedContent})`;
				}

				// Otherwise, just display the text in bold
				return `**${trimmedContent}**`;
			},
		);

		return processedText;
	};

	const thinkingMatch = content.match(/<think>([\s\S]*?)<\/think>/);

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

	const openThinkingMatch = content.match(/<think>([\s\S]*?)$/);
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

	// Preprocess the entire content
	const processedContent = preprocessFocusNodes(content);

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
}
