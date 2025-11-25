"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	BrainCircuitIcon,
	CheckCircle2Icon,
	ChevronDown,
	CircleDashedIcon,
	CircleDotIcon,
	EditIcon,
	LinkIcon,
	LoaderCircleIcon,
	MessageSquareIcon,
	MoveIcon,
	PencilIcon,
	PlayIcon,
	PlusCircleIcon,
	SearchIcon,
	SendIcon,
	SparklesIcon,
	SquarePenIcon,
	Unlink2Icon,
	Wand2Icon,
	WrenchIcon,
	XCircleIcon,
	XIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button, Input, ScrollArea, TextEditor } from "../..";
import { useInvoke } from "../../hooks";
import { IBitTypes, type IBoard } from "../../lib";
import { useBackend } from "../../state/backend-state";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../ui/collapsible";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";

export interface Suggestion {
	node_type: string;
	reason: string;
	connection_description: string;
	position?: { x: number; y: number };
	connections: Array<{
		from_node_id: string;
		from_pin: string;
		to_pin: string;
	}>;
}

/// Agent types for the multi-agent system
export type AgentType = "Explain" | "Edit";

/// Role in the chat conversation
export type ChatRole = "User" | "Assistant";

/// A message in the chat history
export interface ChatMessage {
	role: ChatRole;
	content: string;
}

/// Commands that can be executed on the board
export type BoardCommand =
	| {
			command_type: "AddNode";
			node_type: string;
			ref_id?: string; // Reference ID for this node (e.g., "$0", "$1") used in connections
			position?: { x: number; y: number };
			friendly_name?: string;
			summary?: string;
	  }
	| { command_type: "RemoveNode"; node_id: string; summary?: string }
	| {
			command_type: "ConnectPins";
			from_node: string;
			from_pin: string;
			to_node: string;
			to_pin: string;
			summary?: string;
	  }
	| {
			command_type: "DisconnectPins";
			from_node: string;
			from_pin: string;
			to_node: string;
			to_pin: string;
			summary?: string;
	  }
	| {
			command_type: "UpdateNodePin";
			node_id: string;
			pin_id: string;
			value: unknown;
			summary?: string;
	  }
	| {
			command_type: "MoveNode";
			node_id: string;
			position: { x: number; y: number };
			summary?: string;
	  }
	| {
			command_type: "CreateVariable";
			name: string;
			data_type: string;
			value_type?: string;
			default_value?: unknown;
			description?: string;
			summary?: string;
	  }
	| {
			command_type: "UpdateVariable";
			variable_id: string;
			value: unknown;
			summary?: string;
	  }
	| { command_type: "DeleteVariable"; variable_id: string; summary?: string }
	| {
			command_type: "CreateComment";
			content: string;
			position?: { x: number; y: number };
			color?: string;
			summary?: string;
	  }
	| {
			command_type: "UpdateComment";
			comment_id: string;
			content?: string;
			color?: string;
			summary?: string;
	  }
	| { command_type: "DeleteComment"; comment_id: string; summary?: string }
	| {
			command_type: "CreateLayer";
			name: string;
			color?: string;
			node_ids?: string[];
			summary?: string;
	  }
	| {
			command_type: "AddNodesToLayer";
			layer_id: string;
			node_ids: string[];
			summary?: string;
	  }
	| {
			command_type: "RemoveNodesFromLayer";
			layer_id: string;
			node_ids: string[];
			summary?: string;
	  };

/// Response from the copilot
export interface CopilotResponse {
	agent_type: AgentType;
	message: string;
	commands: BoardCommand[];
	suggestions: Suggestion[];
}

interface PlanStep {
	id: string;
	description: string;
	status: "Pending" | "InProgress" | "Completed" | "Failed";
	tool_name?: string;
}

interface FlowCopilotProps {
	board: IBoard | undefined;
	selectedNodeIds: string[];
	onAcceptSuggestion: (suggestion: Suggestion) => void;
	onFocusNode?: (nodeId: string) => void;
	onGhostNodesChange?: (suggestions: Suggestion[]) => void;
	onExecuteCommands?: (commands: BoardCommand[]) => void;
}

type Mode = "chat" | "autocomplete";

// Helper function to get icon for a command type
function getCommandIcon(cmd: BoardCommand, size = "w-4 h-4") {
	switch (cmd.command_type) {
		case "AddNode":
			return <PlusCircleIcon className={size} />;
		case "RemoveNode":
			return <XCircleIcon className={size} />;
		case "ConnectPins":
			return <LinkIcon className={size} />;
		case "DisconnectPins":
			return <Unlink2Icon className={size} />;
		case "UpdateNodePin":
			return <PencilIcon className={size} />;
		case "MoveNode":
			return <MoveIcon className={size} />;
		case "CreateVariable":
		case "UpdateVariable":
		case "DeleteVariable":
			return <EditIcon className={size} />;
		case "CreateComment":
		case "UpdateComment":
		case "DeleteComment":
			return <MessageSquareIcon className={size} />;
		case "CreateLayer":
		case "AddNodesToLayer":
		case "RemoveNodesFromLayer":
			return <SquarePenIcon className={size} />;
		default:
			return <CircleDotIcon className={size} />;
	}
}

function PlanStepsView({ steps }: { steps: PlanStep[] }) {
	const [expanded, setExpanded] = useState(false);

	if (steps.length === 0) return null;

	const completedCount = steps.filter((s) => s.status === "Completed").length;
	const inProgressCount = steps.filter((s) => s.status === "InProgress").length;

	// Get the most recent step (last one, or last in-progress one)
	const currentStep =
		steps.findLast((s) => s.status === "InProgress") || steps[steps.length - 1];

	const getStatusIcon = (status: PlanStep["status"]) => {
		switch (status) {
			case "Completed":
				return <CheckCircle2Icon className="w-3 h-3 text-green-500" />;
			case "InProgress":
				return (
					<LoaderCircleIcon className="w-3 h-3 text-primary animate-spin" />
				);
			case "Failed":
				return <XCircleIcon className="w-3 h-3 text-destructive" />;
			default:
				return <CircleDashedIcon className="w-3 h-3 text-muted-foreground" />;
		}
	};

	const getToolIcon = (toolName?: string) => {
		if (!toolName) return null;
		switch (toolName) {
			case "catalog_search":
			case "search_by_pin":
			case "filter_category":
				return <SearchIcon className="w-3 h-3" />;
			case "think":
				return <BrainCircuitIcon className="w-3 h-3" />;
			case "focus_node":
				return <CircleDotIcon className="w-3 h-3" />;
			case "emit_commands":
				return <SparklesIcon className="w-3 h-3" />;
			default:
				return <WrenchIcon className="w-3 h-3" />;
		}
	};

	const getStatusColor = (status: PlanStep["status"]) => {
		switch (status) {
			case "Completed":
				return "border-green-500/30 bg-green-500/5";
			case "InProgress":
				return "border-primary/50 bg-primary/10";
			case "Failed":
				return "border-destructive/30 bg-destructive/5";
			default:
				return "border-border/50 bg-muted/20";
		}
	};

	const renderStep = (step: PlanStep) => {
		const isReasoningStep = step.tool_name === "think";
		return (
			<div
				key={step.id}
				className={`flex items-start gap-2 text-xs p-2 rounded-lg border transition-all overflow-hidden ${getStatusColor(step.status)}`}
			>
				<div className="shrink-0 mt-0.5">{getStatusIcon(step.status)}</div>
				<div className="flex-1 min-w-0 overflow-hidden">
					{isReasoningStep ? (
						<Collapsible defaultOpen={step.status === "InProgress"}>
							<CollapsibleTrigger className="flex items-center gap-1.5 w-full text-left group">
								<span className="font-medium text-foreground">Reasoning</span>
								<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180 text-muted-foreground shrink-0" />
							</CollapsibleTrigger>
							<CollapsibleContent>
								<div className="mt-1.5 text-[11px] text-muted-foreground whitespace-pre-wrap wrap-break-word font-mono bg-background/50 rounded p-2 max-h-32 overflow-y-auto overflow-x-hidden">
									{step.description}
								</div>
							</CollapsibleContent>
						</Collapsible>
					) : (
						<div
							className="font-medium text-foreground truncate max-w-[280px]"
							title={step.description}
						>
							{step.description}
						</div>
					)}
				</div>
				{step.tool_name && (
					<div className="shrink-0 flex items-center gap-1 text-[10px] text-muted-foreground px-1.5 py-0.5 rounded bg-background/80">
						{getToolIcon(step.tool_name)}
					</div>
				)}
			</div>
		);
	};

	return (
		<div className="space-y-2 w-full">
			{/* Progress indicator */}
			<div className="flex items-center gap-2 text-xs text-muted-foreground">
				<span className="font-medium px-1.5 py-0.5 rounded-full bg-green-500/20 text-green-600 dark:text-green-400">
					{completedCount}/{steps.length}
				</span>
				<span>
					{inProgressCount > 0
						? "Working..."
						: completedCount === steps.length
							? "Complete"
							: "Planning"}
				</span>
			</div>

			{/* Current/Latest step */}
			{currentStep && renderStep(currentStep)}

			{/* Expand to show all steps */}
			{steps.length > 1 && (
				<Collapsible open={expanded} onOpenChange={setExpanded}>
					<CollapsibleTrigger className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors w-full">
						<ChevronDown
							className={`w-3 h-3 transition-transform duration-200 ${expanded ? "rotate-180" : ""}`}
						/>
						<span>
							{expanded ? "Hide" : "Show"} all {steps.length} steps
						</span>
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="mt-2 space-y-1.5 max-h-48 overflow-y-auto">
							{steps.filter((s) => s.id !== currentStep?.id).map(renderStep)}
						</div>
					</CollapsibleContent>
				</Collapsible>
			)}
		</div>
	);
}

function PendingCommandsView({
	commands,
	onExecute,
	onExecuteSingle,
	onDismiss,
}: {
	commands: BoardCommand[];
	onExecute: () => void;
	onExecuteSingle: (index: number) => void;
	onDismiss: () => void;
}) {
	const [expanded, setExpanded] = useState(false);

	if (commands.length === 0) return null;

	const getCommandEmoji = (cmd: BoardCommand): string => {
		switch (cmd.command_type) {
			case "AddNode":
				return "➕";
			case "RemoveNode":
				return "🗑️";
			case "ConnectPins":
				return "🔗";
			case "DisconnectPins":
				return "✂️";
			case "UpdateNodePin":
				return "✏️";
			case "MoveNode":
				return "📍";
			case "CreateVariable":
			case "UpdateVariable":
			case "DeleteVariable":
				return "📦";
			case "CreateComment":
			case "UpdateComment":
			case "DeleteComment":
				return "💬";
			case "CreateLayer":
			case "AddNodesToLayer":
			case "RemoveNodesFromLayer":
				return "📁";
			default:
				return "⚡";
		}
	};

	const getCommandSummary = (cmd: BoardCommand): string => {
		if (cmd.summary) return cmd.summary;
		switch (cmd.command_type) {
			case "AddNode":
				return `Add ${cmd.friendly_name || cmd.node_type.split("::").pop() || "node"}`;
			case "RemoveNode":
				return "Remove node";
			case "ConnectPins":
				return "Connect";
			case "DisconnectPins":
				return "Disconnect";
			case "UpdateNodePin":
				return "Set value";
			case "MoveNode":
				return "Move";
			default:
				return cmd.command_type;
		}
	};

	// Group commands by type for summary
	const addCount = commands.filter((c) => c.command_type === "AddNode").length;
	const connectCount = commands.filter(
		(c) => c.command_type === "ConnectPins",
	).length;
	const updateCount = commands.filter(
		(c) => c.command_type === "UpdateNodePin",
	).length;
	const otherCount = commands.length - addCount - connectCount - updateCount;

	return (
		<motion.div
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			className="w-full"
		>
			{/* Compact summary bar */}
			<div className="flex items-center gap-2 p-2 rounded-xl bg-linear-to-r from-primary/10 via-purple-500/10 to-primary/10 border border-primary/20">
				<div className="flex items-center gap-1.5 flex-1 min-w-0">
					<SparklesIcon className="w-4 h-4 text-primary shrink-0" />
					<div className="flex items-center gap-1 text-xs font-medium text-foreground overflow-hidden">
						{addCount > 0 && (
							<span className="px-1.5 py-0.5 rounded bg-green-500/20 text-green-600 dark:text-green-400 whitespace-nowrap">
								+{addCount}
							</span>
						)}
						{connectCount > 0 && (
							<span className="px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-600 dark:text-blue-400 whitespace-nowrap">
								🔗{connectCount}
							</span>
						)}
						{updateCount > 0 && (
							<span className="px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-600 dark:text-purple-400 whitespace-nowrap">
								✏️{updateCount}
							</span>
						)}
						{otherCount > 0 && (
							<span className="px-1.5 py-0.5 rounded bg-muted text-muted-foreground whitespace-nowrap">
								+{otherCount}
							</span>
						)}
					</div>
				</div>

				<div className="flex items-center gap-1 shrink-0">
					<Button
						size="sm"
						variant="ghost"
						className="h-7 px-2 text-xs"
						onClick={() => setExpanded(!expanded)}
					>
						<ChevronDown
							className={`w-3 h-3 transition-transform ${expanded ? "rotate-180" : ""}`}
						/>
					</Button>
					<Button
						size="sm"
						className="h-7 px-3 text-xs gap-1 bg-primary hover:bg-primary/90"
						onClick={onExecute}
					>
						<PlayIcon className="w-3 h-3" />
						Apply
					</Button>
					<Button
						size="sm"
						variant="ghost"
						className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive"
						onClick={onDismiss}
					>
						<XIcon className="w-3 h-3" />
					</Button>
				</div>
			</div>

			{/* Expanded details */}
			<AnimatePresence>
				{expanded && (
					<motion.div
						initial={{ height: 0, opacity: 0 }}
						animate={{ height: "auto", opacity: 1 }}
						exit={{ height: 0, opacity: 0 }}
						transition={{ duration: 0.2 }}
						className="overflow-hidden"
					>
						<div className="pt-2 space-y-1 max-h-32 overflow-y-auto">
							{commands.map((cmd, i) => (
								<div
									key={i}
									className="flex items-center gap-2 px-2 py-1.5 rounded-lg bg-muted/50 hover:bg-muted cursor-pointer transition-colors group"
									onClick={() => onExecuteSingle(i)}
								>
									<span className="text-sm shrink-0">
										{getCommandEmoji(cmd)}
									</span>
									<span
										className="text-xs text-foreground truncate flex-1"
										title={getCommandSummary(cmd)}
									>
										{getCommandSummary(cmd)}
									</span>
									<PlayIcon className="w-3 h-3 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
								</div>
							))}
						</div>
					</motion.div>
				)}
			</AnimatePresence>
		</motion.div>
	);
}

function MessageContent({
	content,
	onFocusNode,
	board,
}: {
	content: string;
	onFocusNode?: (nodeId: string) => void;
	board?: IBoard;
}) {
	// Get node name from board by ID
	const getNodeName = (nodeId: string): string => {
		if (!board?.nodes) return "Node";
		const node = board.nodes[nodeId];
		return node?.friendly_name || node?.node_type?.split("::").pop() || "Node";
	};

	// Helper to resolve node ID from name or ID
	const resolveNode = (
		identifier: string,
	): { id: string; name: string } | null => {
		if (!board?.nodes) return null;

		const trimmed = identifier.trim();

		// 1. Check if identifier is a valid ID
		if (board.nodes[trimmed]) {
			return {
				id: trimmed,
				name: getNodeName(trimmed),
			};
		}

		// 2. Search by friendly_name (case-insensitive)
		const nodeByFriendlyName = Object.values(board.nodes).find(
			(n) => n.friendly_name?.toLowerCase() === trimmed.toLowerCase(),
		);
		if (nodeByFriendlyName) {
			return {
				id: nodeByFriendlyName.id,
				name: nodeByFriendlyName.friendly_name || "Node",
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
			(_match: string, content: string) => {
				const trimmedContent = content.trim();
				const resolved = resolveNode(trimmedContent);

				if (resolved) {
					return `[${resolved.name}](focus://${resolved.id})`;
				}

				// Fallback: if it looks like a UUID, assume it's an ID that might exist later
				if (/^[0-9a-fA-F-]{36}$/.test(trimmedContent)) {
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

export function FlowCopilot({
	board,
	selectedNodeIds,
	onAcceptSuggestion,
	onFocusNode,
	onGhostNodesChange,
	onExecuteCommands,
}: FlowCopilotProps) {
	const [isOpen, setIsOpen] = useState(false);
	const [mode, setMode] = useState<Mode>("chat");
	const [input, setInput] = useState("");
	const [messages, setMessages] = useState<
		{
			role: "user" | "assistant";
			content: string;
			agentType?: AgentType;
			executedCommands?: BoardCommand[];
		}[]
	>([]);
	const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
	const [pendingCommands, setPendingCommands] = useState<BoardCommand[]>([]);
	const [loading, setLoading] = useState(false);
	const [currentToolCall, setCurrentToolCall] = useState<string | null>(null);
	const [planSteps, setPlanSteps] = useState<PlanStep[]>([]);
	const [selectedModelId, setSelectedModelId] = useState<string | undefined>(
		undefined,
	);
	const [autocompleteEnabled, setAutocompleteEnabled] = useState(false);
	const backend = useBackend();

	// Debug: Log whenever pendingCommands changes
	useEffect(() => {
		console.log(
			"[FlowCopilot] pendingCommands state changed:",
			pendingCommands.length,
			"commands",
		);
		if (pendingCommands.length > 0) {
			console.log("[FlowCopilot] pendingCommands:", pendingCommands);
		}
	}, [pendingCommands]);

	// Update ghost nodes when suggestions change and autocomplete is enabled
	useEffect(() => {
		if (autocompleteEnabled && onGhostNodesChange) {
			onGhostNodesChange(suggestions);
		} else if (!autocompleteEnabled && onGhostNodesChange) {
			onGhostNodesChange([]);
		}
	}, [autocompleteEnabled, suggestions, onGhostNodesChange]);
	const messagesEndRef = useRef<HTMLDivElement>(null);
	const scrollContainerRef = useRef<HTMLDivElement>(null);
	const [userScrolledUp, setUserScrolledUp] = useState(false);

	// Parse focus node commands from message content
	useEffect(() => {
		if (!onFocusNode) return;

		const lastMessage = messages[messages.length - 1];
		if (lastMessage?.role === "assistant") {
			const focusMatch = lastMessage.content.match(
				/\[Focus:\s*([^\]]+)\]\s*([^\n]+)/g,
			);
			if (focusMatch) {
				// Extract the first node ID
				const match = focusMatch[0].match(/\[Focus:\s*([^\]]+)\]/);
				if (match?.[1]) {
					const nodeId = match[1].trim();
					// Debounce to avoid multiple rapid calls
					const timer = setTimeout(() => {
						onFocusNode(nodeId);
					}, 500);
					return () => clearTimeout(timer);
				}
			}
		}
	}, [messages, onFocusNode]);

	const profile = useInvoke(
		backend.userState.getSettingsProfile,
		backend.userState,
		[],
		isOpen,
	);

	const foundBits = useInvoke(
		backend.bitState.searchBits,
		backend.bitState,
		[
			{
				bit_types: [IBitTypes.Llm, IBitTypes.Vlm],
			},
		],
		isOpen && !!profile.data,
		[profile.data?.hub_profile.id],
	);

	const models = foundBits.data || [];

	useEffect(() => {
		if (!selectedModelId && models.length > 0) {
			// Default to Hosted models, then fall back to GPT-4o, then first available
			const hostedModel = models.find(
				(m) => m.parameters?.provider?.provider_name === "Hosted",
			);
			const gpt4o = models.find((m) => m.id.includes("gpt-4o"));
			const defaultModel = hostedModel || gpt4o || models[0];
			setSelectedModelId(defaultModel.id);
		}
	}, [models, selectedModelId]);

	const scrollToBottom = useCallback(() => {
		if (userScrolledUp) return; // Don't auto-scroll if user scrolled up
		// Use requestAnimationFrame to ensure DOM update is complete
		requestAnimationFrame(() => {
			messagesEndRef.current?.scrollIntoView({
				behavior: "smooth",
				block: "nearest",
			});
		});
	}, [userScrolledUp]);

	// Handle scroll events to detect when user scrolls up
	const handleScroll = useCallback(() => {
		const container = scrollContainerRef.current;
		if (!container) return;

		const { scrollTop, scrollHeight, clientHeight } = container;
		const isAtBottom = scrollHeight - scrollTop - clientHeight < 50; // 50px threshold

		if (isAtBottom) {
			setUserScrolledUp(false);
		} else {
			setUserScrolledUp(true);
		}
	}, []);

	useEffect(() => {
		scrollToBottom();
		// Double check scroll after a short delay to handle any layout shifts (like Collapsible animations)
		const timeout = setTimeout(scrollToBottom, 150);
		return () => clearTimeout(timeout);
	}, [messages, suggestions, loading, scrollToBottom]);

	const handleNewChat = () => {
		setMessages([]);
		setSuggestions([]);
		setPendingCommands([]);
		setPlanSteps([]);
		setInput("");
	};

	const handleExecuteCommands = () => {
		if (onExecuteCommands && pendingCommands.length > 0) {
			onExecuteCommands(pendingCommands);
			// Store executed commands in the last assistant message
			setMessages((prev) => {
				const newMessages = [...prev];
				// Find the last assistant message
				for (let i = newMessages.length - 1; i >= 0; i--) {
					if (newMessages[i].role === "assistant") {
						const existingCommands = newMessages[i].executedCommands || [];
						// Avoid duplicates if possible, though simple concatenation is safer for now
						newMessages[i] = {
							...newMessages[i],
							executedCommands: [...existingCommands, ...pendingCommands],
						};
						console.log(
							`[FlowCopilot] Attached ${pendingCommands.length} commands to message ${i}`,
						);
						break;
					}
				}
				return newMessages;
			});
			setPendingCommands([]);
		}
	};

	const handleExecuteSingle = (index: number) => {
		if (onExecuteCommands && pendingCommands[index]) {
			const command = pendingCommands[index];
			onExecuteCommands([command]);
			// Store executed command in the last assistant message
			setMessages((prev) => {
				const newMessages = [...prev];
				for (let i = newMessages.length - 1; i >= 0; i--) {
					if (newMessages[i].role === "assistant") {
						const existingCommands = newMessages[i].executedCommands || [];
						newMessages[i] = {
							...newMessages[i],
							executedCommands: [...existingCommands, command],
						};
						break;
					}
				}
				return newMessages;
			});
			setPendingCommands((prev) => prev.filter((_, i) => i !== index));
		}
	};

	const handleDismissCommands = () => {
		setPendingCommands([]);
	};

	const handleSubmit = async () => {
		if (!input.trim()) return;

		const userMsg = input;
		setMessages((prev) => [...prev, { role: "user", content: userMsg }]);
		setInput("");
		setLoading(true);
		setSuggestions([]);
		setPendingCommands([]);
		setPlanSteps([]);
		setUserScrolledUp(false); // Reset scroll state when sending new message

		try {
			if (!board) return;

			let currentMessageContent = "";
			// Add an empty assistant message to stream into
			setMessages((prev) => [...prev, { role: "assistant", content: "" }]);

			const onToken = (token: string) => {
				// Check for plan step events
				const planStepMatch = token.match(/<plan_step>([\s\S]*?)<\/plan_step>/);
				if (planStepMatch) {
					try {
						const eventData = JSON.parse(planStepMatch[1]);
						if (eventData.PlanStep) {
							const step = eventData.PlanStep;
							setPlanSteps((prev) => {
								const existingIndex = prev.findIndex((s) => s.id === step.id);
								if (existingIndex >= 0) {
									// Update existing step
									const updated = [...prev];
									updated[existingIndex] = step;
									return updated;
								}
								// Add new step
								return [...prev, step];
							});
						}
					} catch {
						// Not valid JSON, ignore
					}
					// Don't add plan step tags to the message
					return;
				}

				// Check for commands (don't add to visible message)
				if (token.includes("<commands>") && token.includes("</commands>")) {
					// Commands are handled separately by the response
					return;
				}

				// Check for tool calls to show working indicator
				if (token.includes("tool_call:")) {
					const match = token.match(/tool_call:(\w+)/);
					if (match) {
						setCurrentToolCall(match[1]);
					}
					return;
				}
				if (token.includes("tool_result:")) {
					setCurrentToolCall(null);
					return;
				}

				currentMessageContent += token;
				setMessages((prev) => {
					const newMessages = [...prev];
					const lastMessage = newMessages[newMessages.length - 1];
					if (lastMessage && lastMessage.role === "assistant") {
						lastMessage.content = currentMessageContent;
					}
					return newMessages;
				});
			};

			// Use the new chat method which handles multi-agent routing
			// Convert messages to history format for backend
			const chatHistory: ChatMessage[] = messages.map((m) => ({
				role: m.role === "user" ? "User" : "Assistant",
				content: m.content,
			}));

			const response = await backend.boardState.flowpilot_chat(
				board,
				selectedNodeIds,
				userMsg,
				chatHistory,
				onToken,
				selectedModelId,
			);

			// Update the final message with agent type
			setMessages((prev) => {
				const newMessages = [...prev];
				const lastMessage = newMessages[newMessages.length - 1];
				if (lastMessage && lastMessage.role === "assistant") {
					lastMessage.agentType = response.agent_type;
					// If the streamed content is empty, use the response message
					if (!lastMessage.content.trim()) {
						lastMessage.content = response.message;
					}
				}
				return newMessages;
			});

			// Handle commands from the Edit agent
			console.log(`[FlowCopilot] === Response received ===`);
			console.log(`[FlowCopilot] agent_type: ${response.agent_type}`);
			console.log(`[FlowCopilot] commands: ${response.commands.length}`);
			console.log(`[FlowCopilot] message length: ${response.message.length}`);
			console.log(`[FlowCopilot] suggestions: ${response.suggestions.length}`);

			// Always set pending commands from response (even if empty, for debugging)
			console.log(
				"[FlowCopilot] Setting pendingCommands to:",
				response.commands,
			);
			setPendingCommands(response.commands);

			if (response.commands.length > 0) {
				console.log("[FlowCopilot] Commands to execute:");
				response.commands.forEach((cmd, idx) => {
					console.log(`  [${idx}] ${cmd.command_type}:`, cmd);
				});
			}

			// Handle suggestions (for backward compatibility)
			if (response.suggestions.length > 0) {
				setSuggestions(response.suggestions);
			}
		} catch (e) {
			console.error(e);
			setMessages((prev) => [
				...prev,
				{
					role: "assistant",
					content: "Sorry, I encountered an error processing your request.",
				},
			]);
		} finally {
			setLoading(false);
			setCurrentToolCall(null);
		}
	};

	if (!isOpen) {
		return (
			<motion.div
				initial={{ scale: 0, opacity: 0 }}
				animate={{ scale: 1, opacity: 1 }}
				className="absolute bottom-6 right-6 z-50"
			>
				<div className="relative">
					<Button
						className="rounded-full w-14 h-14 p-0 shadow-xl bg-linear-to-r from-primary to-purple-600 hover:scale-105 transition-transform"
						onClick={() => setIsOpen(true)}
					>
						<SparklesIcon className="w-6 h-6 text-white" />
					</Button>
					{autocompleteEnabled && (
						<div className="absolute -top-1 -right-1 w-5 h-5 bg-green-500 border-2 border-background rounded-full flex items-center justify-center">
							<Wand2Icon className="w-3 h-3 text-white" />
						</div>
					)}
				</div>
			</motion.div>
		);
	}

	return (
		<AnimatePresence>
			{isOpen && (
				<motion.div
					initial={{ opacity: 0, y: 20, scale: 0.95 }}
					animate={{ opacity: 1, y: 0, scale: 1 }}
					exit={{ opacity: 0, y: 20, scale: 0.95 }}
					transition={{ duration: 0.2 }}
					className="absolute bottom-6 right-6 z-50"
					style={{ width: 420, height: 600 }}
				>
					{/* Animated wavy border */}
					{loading && (
						<div className="absolute -inset-[2px] rounded-[26px] overflow-hidden">
							{/* Animated gradient that flows along the border */}
							<svg
								className="absolute inset-0 w-full h-full"
								viewBox="0 0 420 600"
								preserveAspectRatio="none"
							>
								<defs>
									<linearGradient
										id="flowGradient"
										x1="0%"
										y1="0%"
										x2="100%"
										y2="100%"
									>
										<stop offset="0%" stopColor="#8b5cf6">
											<animate
												attributeName="stop-color"
												values="#8b5cf6;#ec4899;#3b82f6;#10b981;#8b5cf6"
												dur="3s"
												repeatCount="indefinite"
											/>
										</stop>
										<stop offset="50%" stopColor="#ec4899">
											<animate
												attributeName="stop-color"
												values="#ec4899;#3b82f6;#10b981;#8b5cf6;#ec4899"
												dur="3s"
												repeatCount="indefinite"
											/>
										</stop>
										<stop offset="100%" stopColor="#3b82f6">
											<animate
												attributeName="stop-color"
												values="#3b82f6;#10b981;#8b5cf6;#ec4899;#3b82f6"
												dur="3s"
												repeatCount="indefinite"
											/>
										</stop>
									</linearGradient>
									{/* Wavy displacement filter */}
									<filter
										id="wavyFilter"
										x="-20%"
										y="-20%"
										width="140%"
										height="140%"
									>
										<feTurbulence
											type="turbulence"
											baseFrequency="0.02"
											numOctaves="3"
											result="turbulence"
										>
											<animate
												attributeName="baseFrequency"
												values="0.02;0.04;0.02"
												dur="2s"
												repeatCount="indefinite"
											/>
										</feTurbulence>
										<feDisplacementMap
											in="SourceGraphic"
											in2="turbulence"
											scale="4"
											xChannelSelector="R"
											yChannelSelector="G"
										/>
									</filter>
								</defs>
								{/* Border path with wavy effect */}
								<rect
									x="2"
									y="2"
									width="416"
									height="596"
									rx="24"
									ry="24"
									fill="none"
									stroke="url(#flowGradient)"
									strokeWidth="4"
									filter="url(#wavyFilter)"
								/>
							</svg>
							{/* Glow effect */}
							<div
								className="absolute inset-0 rounded-[26px] opacity-40 blur-md"
								style={{
									background:
										"linear-gradient(45deg, #8b5cf6, #ec4899, #3b82f6, #10b981)",
									backgroundSize: "400% 400%",
									animation: "gradient-shift 3s ease infinite",
								}}
							/>
							<style>{`
								@keyframes gradient-shift {
									0% { background-position: 0% 50%; }
									50% { background-position: 100% 50%; }
									100% { background-position: 0% 50%; }
								}
							`}</style>
						</div>
					)}

					{/* Main container - sits on top */}
					<div className="absolute inset-0 bg-background rounded-3xl border border-border/50 flex flex-col overflow-hidden shadow-2xl">
						{/* Header */}
						<div className="flex flex-col border-b border-border/50 bg-linear-to-r from-primary/5 via-purple-500/5 to-primary/5 relative">
							<div className="p-5 pb-3 flex justify-between items-center">
								<div className="flex items-center gap-3">
									<div className="p-2.5 bg-linear-to-br from-primary to-purple-600 rounded-xl shadow-lg shadow-primary/20">
										<SparklesIcon className="w-5 h-5 text-white" />
									</div>
									<div>
										<h3 className="font-semibold text-base">FlowPilot</h3>
										<p className="text-xs text-muted-foreground flex items-center gap-1">
											{loading ? (
												<>
													<LoaderCircleIcon className="w-3 h-3 animate-spin text-primary" />
													<span className="text-primary font-medium">
														Working...
													</span>
												</>
											) : (
												<>
													<span className="w-1.5 h-1.5 bg-green-500 rounded-full animate-pulse" />
													AI Assistant
												</>
											)}
										</p>
									</div>
								</div>
								<div className="flex items-center gap-1.5">
									<Button
										variant={autocompleteEnabled ? "default" : "ghost"}
										size="icon"
										className={`h-9 w-9 rounded-xl transition-all duration-200 ${
											autocompleteEnabled
												? "bg-primary/90 hover:bg-primary shadow-md shadow-primary/20"
												: "hover:bg-accent/50"
										}`}
										onClick={() => setAutocompleteEnabled(!autocompleteEnabled)}
										title={
											autocompleteEnabled
												? "Disable Autocomplete"
												: "Enable Autocomplete"
										}
									>
										<Wand2Icon
											className={`w-4 h-4 ${autocompleteEnabled ? "text-primary-foreground" : ""}`}
										/>
									</Button>
									<Button
										variant="ghost"
										size="icon"
										className="h-9 w-9 rounded-xl hover:bg-accent/50 transition-all duration-200"
										onClick={handleNewChat}
										title="New Chat"
									>
										<SquarePenIcon className="w-4 h-4" />
									</Button>
									<Button
										variant="ghost"
										size="icon"
										className="h-9 w-9 rounded-xl hover:bg-destructive/10 hover:text-destructive transition-all duration-200"
										onClick={() => setIsOpen(false)}
									>
										<XIcon className="w-4 h-4" />
									</Button>
								</div>
							</div>

							<div className="px-5 pb-4 flex items-center gap-2">
								<div className="flex-1">
									<Select
										value={selectedModelId}
										onValueChange={setSelectedModelId}
									>
										<SelectTrigger className="h-9 text-xs bg-background/80 backdrop-blur-sm border-border/50 hover:border-primary/30 transition-all duration-200 rounded-xl focus:ring-2 focus:ring-primary/20">
											<SelectValue placeholder="Select Model" />
										</SelectTrigger>
										<SelectContent className="rounded-xl">
											{models.map((model) => (
												<SelectItem
													key={model.id}
													value={model.id}
													className="text-xs rounded-lg"
												>
													{model.meta?.en?.name || model.id}
												</SelectItem>
											))}
										</SelectContent>
									</Select>
								</div>
							</div>

							{/* Loading Indicator Line */}
							{loading && (
								<motion.div
									className="absolute bottom-0 left-0 right-0 h-0.5 bg-linear-to-r from-transparent via-primary to-transparent"
									initial={{ x: "-100%" }}
									animate={{ x: "100%" }}
									transition={{
										repeat: Number.POSITIVE_INFINITY,
										duration: 1.5,
										ease: "easeInOut",
									}}
								/>
							)}
						</div>

						{/* Chat Area */}
						<ScrollArea
							className="flex-1 p-5 flex flex-col max-h-full overflow-auto"
							viewportRef={scrollContainerRef}
							onScroll={handleScroll}
						>
							<div className="space-y-3 min-w-0">
								{messages.length === 0 && (
									<div className="text-center text-muted-foreground py-16 space-y-3">
										<div className="relative inline-block">
											<div className="absolute inset-0 bg-primary/20 blur-2xl rounded-full" />
											<SparklesIcon className="w-12 h-12 mx-auto relative text-primary/40" />
										</div>
										<p className="text-sm font-medium">
											How can I help you build your flow today?
										</p>
										<p className="text-xs text-muted-foreground/60">
											Ask me anything about your workflow
										</p>
									</div>
								)}

								{messages.map((m, i) => (
									<motion.div
										initial={{ opacity: 0, y: 10 }}
										animate={{ opacity: 1, y: 0 }}
										key={i}
										className={`flex min-w-0 ${m.role === "user" ? "justify-end" : "justify-start"}`}
									>
										<div
											className={`p-3.5 rounded-2xl text-sm wrap-break-word overflow-hidden transition-all duration-200 min-w-0 ${
												m.role === "user"
													? "bg-linear-to-br from-primary to-primary/90 text-primary-foreground rounded-br-sm max-w-[85%] shadow-md shadow-primary/10 [&_a]:text-primary-foreground [&_a]:underline [&_a]:underline-offset-2"
													: "bg-muted/60 backdrop-blur-sm rounded-bl-sm max-w-full border border-border/30"
											}`}
										>
											{m.role === "assistant" && m.agentType && (
												<div className="flex items-center gap-1.5 mb-2 pb-2 border-b border-border/30">
													{m.agentType === "Explain" ? (
														<BrainCircuitIcon className="w-3.5 h-3.5 text-blue-500" />
													) : (
														<EditIcon className="w-3.5 h-3.5 text-amber-500" />
													)}
													<span
														className={`text-xs font-medium ${
															m.agentType === "Explain"
																? "text-blue-500"
																: "text-amber-500"
														}`}
													>
														{m.agentType === "Explain"
															? "Explain Agent"
															: "Edit Agent"}
													</span>
												</div>
											)}
											{m.content ? (
												<MessageContent
													content={m.content}
													onFocusNode={onFocusNode}
													board={board}
												/>
											) : (
												<div className="space-y-3">
													<div className="flex items-center gap-2 text-muted-foreground">
														<div className="flex gap-1">
															<span
																className="w-2 h-2 bg-primary/60 rounded-full animate-bounce"
																style={{ animationDelay: "0ms" }}
															/>
															<span
																className="w-2 h-2 bg-primary/60 rounded-full animate-bounce"
																style={{ animationDelay: "150ms" }}
															/>
															<span
																className="w-2 h-2 bg-primary/60 rounded-full animate-bounce"
																style={{ animationDelay: "300ms" }}
															/>
														</div>
														<span className="text-xs font-medium">
															Thinking...
														</span>
													</div>
													{planSteps.length > 0 && (
														<PlanStepsView steps={planSteps} />
													)}
												</div>
											)}
											{/* Show executed commands */}
											{m.executedCommands && m.executedCommands.length > 0 && (
												<div className="mt-3 pt-3 border-t border-border/30">
													<div className="flex items-center gap-1.5 mb-2 text-xs text-muted-foreground">
														<CheckCircle2Icon className="w-3 h-3 text-green-500" />
														<span className="font-medium">
															Executed {m.executedCommands.length} command
															{m.executedCommands.length > 1 ? "s" : ""}
														</span>
													</div>
													<div className="space-y-1">
														{m.executedCommands.map((cmd, cmdIndex) => (
															<div
																key={cmdIndex}
																className="text-xs bg-green-500/10 text-green-700 dark:text-green-400 px-2 py-1 rounded flex items-center gap-1.5 max-w-full overflow-hidden"
																title={cmd.summary || cmd.command_type}
															>
																<span className="shrink-0">
																	{getCommandIcon(cmd)}
																</span>
																<span className="truncate max-w-[200px]">
																	{cmd.summary || cmd.command_type}
																</span>
															</div>
														))}
													</div>
												</div>
											)}
										</div>
									</motion.div>
								))}

								{/* Pending Commands UI */}
								{pendingCommands.length > 0 && (
									<PendingCommandsView
										commands={pendingCommands}
										onExecute={handleExecuteCommands}
										onExecuteSingle={handleExecuteSingle}
										onDismiss={handleDismissCommands}
									/>
								)}

								{suggestions.length > 0 && (
									<motion.div
										initial={{ opacity: 0, y: 10 }}
										animate={{ opacity: 1, y: 0 }}
										className="space-y-2.5 pl-3 border-l-2 border-primary/30 ml-2"
									>
										<p className="text-xs font-semibold text-foreground/70 mb-2 flex items-center gap-1.5">
											<Wand2Icon className="w-3.5 h-3.5 text-primary" />
											Suggestions
										</p>
										{suggestions.map((s, i) => (
											<div
												key={i}
												className="group border border-border/50 bg-card/80 backdrop-blur-sm hover:bg-accent/30 hover:border-primary/40 p-3.5 rounded-xl cursor-pointer transition-all duration-200 hover:shadow-md hover:shadow-primary/5"
												onClick={() => onAcceptSuggestion(s)}
											>
												<div className="flex items-center justify-between mb-1.5">
													<span className="font-semibold text-sm text-primary">
														{s.node_type}
													</span>
													<Wand2Icon className="w-3.5 h-3.5 opacity-0 group-hover:opacity-100 transition-opacity text-primary" />
												</div>
												<div className="text-xs text-muted-foreground leading-relaxed">
													{s.reason}
												</div>
											</div>
										))}
									</motion.div>
								)}

								{loading &&
									messages.length > 0 &&
									messages[messages.length - 1].role === "user" && (
										<motion.div
											initial={{ opacity: 0, y: 10 }}
											animate={{ opacity: 1, y: 0 }}
											className="flex justify-start"
										>
											<div className="bg-muted/60 backdrop-blur-sm border border-border/30 rounded-2xl rounded-bl-sm px-4 py-3 flex items-center gap-2.5">
												<div className="flex gap-1">
													<span
														className="w-2 h-2 bg-primary rounded-full animate-bounce"
														style={{ animationDelay: "0ms" }}
													/>
													<span
														className="w-2 h-2 bg-primary rounded-full animate-bounce"
														style={{ animationDelay: "150ms" }}
													/>
													<span
														className="w-2 h-2 bg-primary rounded-full animate-bounce"
														style={{ animationDelay: "300ms" }}
													/>
												</div>
												<span className="text-xs text-muted-foreground font-medium">
													{currentToolCall ? (
														<span className="flex items-center gap-1.5">
															<WrenchIcon className="w-3 h-3 animate-spin" />
															Using {currentToolCall.replace(/_/g, " ")}...
														</span>
													) : (
														"Thinking..."
													)}
												</span>
											</div>
										</motion.div>
									)}
								<div ref={messagesEndRef} />
							</div>
						</ScrollArea>

						{/* Input Area */}
						<div className="p-5 bg-background/80 backdrop-blur-sm border-t border-border/50">
							<div className="relative flex items-center gap-2">
								<Input
									value={input}
									onChange={(e) => setInput(e.target.value)}
									onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
									placeholder={
										mode === "chat"
											? "Describe what you want to change..."
											: "What node should come next?"
									}
									className="pr-11 rounded-xl border-border/50 focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:border-primary/50 bg-background/80 backdrop-blur-sm transition-all duration-200"
								/>
								<Button
									size="icon"
									onClick={handleSubmit}
									disabled={loading || !input.trim()}
									className="absolute right-0 h-9 w-9 rounded-lg shadow-md bg-linear-to-br from-primary to-purple-600 hover:shadow-lg hover:shadow-primary/20 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
								>
									<SendIcon className="w-4 h-4" />
								</Button>
							</div>
						</div>
					</div>
				</motion.div>
			)}
		</AnimatePresence>
	);
}
