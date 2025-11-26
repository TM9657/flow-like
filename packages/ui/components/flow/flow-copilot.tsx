"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
	ArrowDownIcon,
	BotIcon,
	BrainCircuitIcon,
	CheckCircle2Icon,
	ChevronDown,
	CircleDashedIcon,
	CircleDotIcon,
	ClockIcon,
	CpuIcon,
	EditIcon,
	LayersIcon,
	LinkIcon,
	ListChecksIcon,
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
	TargetIcon,
	Unlink2Icon,
	Wand2Icon,
	WrenchIcon,
	XCircleIcon,
	XIcon,
	ZapIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../ui/tooltip";

// Re-export types from the shared schema file for backwards compatibility
export type {
	Suggestion,
	AgentType,
	ChatRole,
	ChatMessage,
	BoardCommand,
	CopilotResponse,
	PlanStep,
} from "../../lib/schema/flow/copilot";

import type {
	Suggestion,
	AgentType,
	ChatMessage,
	BoardCommand,
	CopilotResponse,
	PlanStep,
} from "../../lib/schema/flow/copilot";

type LoadingPhase =
	| "initializing"
	| "analyzing"
	| "searching"
	| "reasoning"
	| "generating"
	| "finalizing";

interface FlowCopilotProps {
	board: IBoard | undefined;
	selectedNodeIds: string[];
	onAcceptSuggestion: (suggestion: Suggestion) => void;
	onFocusNode?: (nodeId: string) => void;
	onGhostNodesChange?: (suggestions: Suggestion[]) => void;
	onExecuteCommands?: (commands: BoardCommand[]) => void;
}

type Mode = "chat" | "autocomplete";

const LOADING_PHASES: Record<LoadingPhase, { label: string; icon: React.ReactNode; color: string }> = {
	initializing: { label: "Initializing...", icon: <CpuIcon className="w-3.5 h-3.5" />, color: "text-blue-500" },
	analyzing: { label: "Analyzing your flow...", icon: <SearchIcon className="w-3.5 h-3.5" />, color: "text-violet-500" },
	searching: { label: "Searching catalog...", icon: <SearchIcon className="w-3.5 h-3.5" />, color: "text-cyan-500" },
	reasoning: { label: "Thinking...", icon: <BrainCircuitIcon className="w-3.5 h-3.5" />, color: "text-amber-500" },
	generating: { label: "Generating response...", icon: <SparklesIcon className="w-3.5 h-3.5" />, color: "text-pink-500" },
	finalizing: { label: "Finalizing...", icon: <CheckCircle2Icon className="w-3.5 h-3.5" />, color: "text-green-500" },
};

function StatusPill({ phase, elapsed }: { phase: LoadingPhase; elapsed: number }) {
	const phaseInfo = LOADING_PHASES[phase];
	return (
		<motion.div
			initial={{ opacity: 0, scale: 0.9 }}
			animate={{ opacity: 1, scale: 1 }}
			className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-background/80 backdrop-blur-sm border border-border/50 ${phaseInfo.color}`}
		>
			<motion.div
				animate={{ rotate: phase === "reasoning" ? 360 : 0 }}
				transition={{ duration: 2, repeat: phase === "reasoning" ? Infinity : 0, ease: "linear" }}
			>
				{phaseInfo.icon}
			</motion.div>
			<span>{phaseInfo.label}</span>
			{elapsed > 0 && (
				<span className="text-muted-foreground/60 tabular-nums">{elapsed}s</span>
			)}
		</motion.div>
	);
}

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
	const progress = steps.length > 0 ? (completedCount / steps.length) * 100 : 0;

	const currentStep =
		steps.findLast((s) => s.status === "InProgress") || steps[steps.length - 1];

	const getStatusIcon = (status: PlanStep["status"]) => {
		switch (status) {
			case "Completed":
				return (
					<motion.div initial={{ scale: 0 }} animate={{ scale: 1 }}>
						<CheckCircle2Icon className="w-3.5 h-3.5 text-green-500" />
					</motion.div>
				);
			case "InProgress":
				return (
					<div className="relative">
						<div className="absolute inset-0 bg-primary/30 rounded-full animate-ping" />
						<LoaderCircleIcon className="w-3.5 h-3.5 text-primary animate-spin relative" />
					</div>
				);
			case "Failed":
				return <XCircleIcon className="w-3.5 h-3.5 text-destructive" />;
			default:
				return <CircleDashedIcon className="w-3.5 h-3.5 text-muted-foreground/50" />;
		}
	};

	const getToolIcon = (toolName?: string) => {
		if (!toolName) return <ZapIcon className="w-3 h-3" />;
		switch (toolName) {
			case "catalog_search":
			case "search_by_pin":
			case "filter_category":
				return <SearchIcon className="w-3 h-3" />;
			case "think":
				return <BrainCircuitIcon className="w-3 h-3" />;
			case "focus_node":
				return <TargetIcon className="w-3 h-3" />;
			case "emit_commands":
				return <SparklesIcon className="w-3 h-3" />;
			default:
				return <WrenchIcon className="w-3 h-3" />;
		}
	};

	const getToolLabel = (toolName?: string) => {
		if (!toolName) return "Processing";
		switch (toolName) {
			case "catalog_search": return "Searching";
			case "search_by_pin": return "Finding pins";
			case "filter_category": return "Filtering";
			case "think": return "Reasoning";
			case "focus_node": return "Focusing";
			case "emit_commands": return "Building";
			default: return toolName.replace(/_/g, " ");
		}
	};

	return (
		<motion.div
			initial={{ opacity: 0, y: 5 }}
			animate={{ opacity: 1, y: 0 }}
			className="space-y-2.5 w-full"
		>
			{/* Progress bar with animated fill */}
			<div className="flex items-center gap-3">
				<div className="flex-1 h-1.5 bg-muted/50 rounded-full overflow-hidden">
					<motion.div
						className="h-full bg-linear-to-r from-primary via-violet-500 to-primary rounded-full"
						initial={{ width: 0 }}
						animate={{ width: `${progress}%` }}
						transition={{ duration: 0.5, ease: "easeOut" }}
					/>
				</div>
				<span className="text-[10px] font-medium text-muted-foreground tabular-nums shrink-0">
					{completedCount}/{steps.length}
				</span>
			</div>

			{/* Current step with enhanced styling */}
			{currentStep && (
				<motion.div
					key={currentStep.id}
					initial={{ opacity: 0, x: -10 }}
					animate={{ opacity: 1, x: 0 }}
					className={`relative overflow-hidden rounded-xl border transition-all ${
						currentStep.status === "InProgress"
							? "border-primary/40 bg-linear-to-r from-primary/10 via-violet-500/5 to-transparent"
							: currentStep.status === "Completed"
								? "border-green-500/30 bg-green-500/5"
								: "border-border/50 bg-muted/30"
					}`}
				>
					{currentStep.status === "InProgress" && (
						<motion.div
							className="absolute inset-0 bg-linear-to-r from-primary/10 via-transparent to-primary/10"
							animate={{ x: ["0%", "100%", "0%"] }}
							transition={{ duration: 3, repeat: Infinity, ease: "linear" }}
						/>
					)}
					<div className="relative p-3 flex items-start gap-2.5">
						<div className="shrink-0 mt-0.5">
							{getStatusIcon(currentStep.status)}
						</div>
						<div className="flex-1 min-w-0">
							{currentStep.tool_name === "think" ? (
								<Collapsible defaultOpen={currentStep.status === "InProgress"}>
									<CollapsibleTrigger className="flex items-center gap-2 w-full text-left group">
										<span className="text-xs font-medium text-foreground">Reasoning</span>
										<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180 text-muted-foreground shrink-0" />
									</CollapsibleTrigger>
									<CollapsibleContent>
										<div className="mt-2 text-[11px] text-muted-foreground whitespace-pre-wrap font-mono bg-background/60 rounded-lg p-2.5 max-h-28 overflow-y-auto border border-border/30">
											{currentStep.description}
											{currentStep.status === "InProgress" && (
												<span className="inline-block w-1.5 h-3 ml-0.5 bg-primary/60 animate-pulse rounded-sm" />
											)}
										</div>
									</CollapsibleContent>
								</Collapsible>
							) : (
								<div className="flex items-center gap-2">
									<span className="text-xs font-medium text-foreground truncate">
										{currentStep.description}
									</span>
								</div>
							)}
						</div>
						{currentStep.tool_name && (
							<div className="shrink-0 flex items-center gap-1 text-[10px] text-muted-foreground px-2 py-1 rounded-lg bg-background/60 border border-border/30">
								{getToolIcon(currentStep.tool_name)}
								<span className="hidden sm:inline">{getToolLabel(currentStep.tool_name)}</span>
							</div>
						)}
					</div>
				</motion.div>
			)}

			{/* Expandable history */}
			{steps.length > 1 && (
				<Collapsible open={expanded} onOpenChange={setExpanded}>
					<CollapsibleTrigger className="flex items-center gap-2 text-[11px] text-muted-foreground hover:text-foreground transition-colors w-full py-1">
						<ChevronDown className={`w-3 h-3 transition-transform duration-200 ${expanded ? "rotate-180" : ""}`} />
						<span>{expanded ? "Hide" : "Show"} {steps.length - 1} previous step{steps.length > 2 ? "s" : ""}</span>
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="mt-2 space-y-1.5 pl-1">
							{steps.filter((s) => s.id !== currentStep?.id).map((step) => (
								<motion.div
									key={step.id}
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									className="flex items-center gap-2 text-[11px] text-muted-foreground py-1"
								>
									{getStatusIcon(step.status)}
									<span className="truncate flex-1">{step.description}</span>
								</motion.div>
							))}
						</div>
					</CollapsibleContent>
				</Collapsible>
			)}
		</motion.div>
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
	const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

	if (commands.length === 0) return null;

	const getCommandColor = (cmd: BoardCommand): string => {
		switch (cmd.command_type) {
			case "AddNode":
				return "from-green-500/20 to-emerald-500/10 border-green-500/30";
			case "RemoveNode":
				return "from-red-500/20 to-rose-500/10 border-red-500/30";
			case "ConnectPins":
				return "from-blue-500/20 to-cyan-500/10 border-blue-500/30";
			case "DisconnectPins":
				return "from-orange-500/20 to-amber-500/10 border-orange-500/30";
			case "UpdateNodePin":
				return "from-violet-500/20 to-purple-500/10 border-violet-500/30";
			default:
				return "from-primary/20 to-primary/10 border-primary/30";
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
				return `Connect ${cmd.from_pin} → ${cmd.to_pin}`;
			case "DisconnectPins":
				return "Disconnect pins";
			case "UpdateNodePin":
				return `Set ${cmd.pin_id}`;
			case "MoveNode":
				return "Move node";
			default:
				return cmd.command_type.replace(/([A-Z])/g, " $1").trim();
		}
	};

	const addCount = commands.filter((c) => c.command_type === "AddNode").length;
	const connectCount = commands.filter((c) => c.command_type === "ConnectPins").length;
	const updateCount = commands.filter((c) => c.command_type === "UpdateNodePin").length;
	const removeCount = commands.filter((c) => c.command_type === "RemoveNode").length;

	return (
		<motion.div
			initial={{ opacity: 0, y: 15, scale: 0.98 }}
			animate={{ opacity: 1, y: 0, scale: 1 }}
			transition={{ type: "spring", stiffness: 400, damping: 25 }}
			className="w-full"
		>
			{/* Enhanced summary card */}
			<div className="relative overflow-hidden rounded-2xl border border-primary/20 bg-linear-to-br from-primary/5 via-violet-500/5 to-pink-500/5">
				{/* Animated background shimmer */}
				<motion.div
					className="absolute inset-0 bg-linear-to-r from-transparent via-white/5 to-transparent"
					animate={{ x: ["-100%", "200%"] }}
					transition={{ duration: 2, repeat: Infinity, repeatDelay: 1 }}
				/>

				<div className="relative p-3.5">
					<div className="flex items-center justify-between gap-3 mb-3">
						<div className="flex items-center gap-2.5">
							<div className="relative">
								<div className="absolute inset-0 bg-primary/30 rounded-lg blur-md" />
								<div className="relative p-1.5 bg-linear-to-br from-primary to-violet-600 rounded-lg">
									<SparklesIcon className="w-4 h-4 text-white" />
								</div>
							</div>
							<div>
								<div className="text-sm font-semibold text-foreground">
									Ready to Apply
								</div>
								<div className="text-[11px] text-muted-foreground">
									{commands.length} change{commands.length > 1 ? "s" : ""} pending
								</div>
							</div>
						</div>

						<div className="flex items-center gap-1.5">
							<Tooltip>
								<TooltipTrigger asChild>
									<Button
										size="sm"
										variant="ghost"
										className="h-8 w-8 p-0 rounded-lg hover:bg-background/60"
										onClick={() => setExpanded(!expanded)}
									>
										<ChevronDown className={`w-4 h-4 transition-transform duration-200 ${expanded ? "rotate-180" : ""}`} />
									</Button>
								</TooltipTrigger>
								<TooltipContent side="top" className="text-xs">
									{expanded ? "Collapse" : "Expand"} details
								</TooltipContent>
							</Tooltip>

							<Button
								size="sm"
								className="h-8 px-4 text-xs font-medium gap-1.5 bg-linear-to-r from-primary to-violet-600 hover:from-primary/90 hover:to-violet-600/90 shadow-lg shadow-primary/20 rounded-lg"
								onClick={onExecute}
							>
								<PlayIcon className="w-3.5 h-3.5" />
								Apply All
							</Button>

							<Tooltip>
								<TooltipTrigger asChild>
									<Button
										size="sm"
										variant="ghost"
										className="h-8 w-8 p-0 rounded-lg text-muted-foreground hover:text-destructive hover:bg-destructive/10"
										onClick={onDismiss}
									>
										<XIcon className="w-4 h-4" />
									</Button>
								</TooltipTrigger>
								<TooltipContent side="top" className="text-xs">
									Dismiss changes
								</TooltipContent>
							</Tooltip>
						</div>
					</div>

					{/* Command type badges */}
					<div className="flex flex-wrap gap-1.5">
						{addCount > 0 && (
							<motion.span
								initial={{ scale: 0 }}
								animate={{ scale: 1 }}
								className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-green-500/15 text-green-600 dark:text-green-400 border border-green-500/20"
							>
								<PlusCircleIcon className="w-3 h-3" />
								{addCount} node{addCount > 1 ? "s" : ""}
							</motion.span>
						)}
						{connectCount > 0 && (
							<motion.span
								initial={{ scale: 0 }}
								animate={{ scale: 1 }}
								transition={{ delay: 0.05 }}
								className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/15 text-blue-600 dark:text-blue-400 border border-blue-500/20"
							>
								<LinkIcon className="w-3 h-3" />
								{connectCount} connection{connectCount > 1 ? "s" : ""}
							</motion.span>
						)}
						{updateCount > 0 && (
							<motion.span
								initial={{ scale: 0 }}
								animate={{ scale: 1 }}
								transition={{ delay: 0.1 }}
								className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-violet-500/15 text-violet-600 dark:text-violet-400 border border-violet-500/20"
							>
								<PencilIcon className="w-3 h-3" />
								{updateCount} update{updateCount > 1 ? "s" : ""}
							</motion.span>
						)}
						{removeCount > 0 && (
							<motion.span
								initial={{ scale: 0 }}
								animate={{ scale: 1 }}
								transition={{ delay: 0.15 }}
								className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-red-500/15 text-red-600 dark:text-red-400 border border-red-500/20"
							>
								<XCircleIcon className="w-3 h-3" />
								{removeCount} removal{removeCount > 1 ? "s" : ""}
							</motion.span>
						)}
					</div>
				</div>
			</div>

			{/* Expanded command list */}
			<AnimatePresence>
				{expanded && (
					<motion.div
						initial={{ height: 0, opacity: 0 }}
						animate={{ height: "auto", opacity: 1 }}
						exit={{ height: 0, opacity: 0 }}
						transition={{ duration: 0.2 }}
						className="overflow-hidden"
					>
						<div className="pt-2 space-y-1.5 max-h-40 overflow-y-auto">
							{commands.map((cmd, i) => (
								<motion.div
									key={i}
									initial={{ opacity: 0, x: -10 }}
									animate={{ opacity: 1, x: 0 }}
									transition={{ delay: i * 0.03 }}
									className={`group relative flex items-center gap-2.5 p-2.5 rounded-xl bg-linear-to-r ${getCommandColor(cmd)} border cursor-pointer transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]`}
									onClick={() => onExecuteSingle(i)}
									onMouseEnter={() => setHoveredIndex(i)}
									onMouseLeave={() => setHoveredIndex(null)}
								>
									<div className="shrink-0">
										{getCommandIcon(cmd, "w-4 h-4")}
									</div>
									<span className="text-xs font-medium text-foreground truncate flex-1">
										{getCommandSummary(cmd)}
									</span>
									<motion.div
										initial={{ opacity: 0, scale: 0.8 }}
										animate={{
											opacity: hoveredIndex === i ? 1 : 0,
											scale: hoveredIndex === i ? 1 : 0.8
										}}
										className="shrink-0 p-1 rounded-md bg-primary/20"
									>
										<PlayIcon className="w-3 h-3 text-primary" />
									</motion.div>
								</motion.div>
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
			planSteps?: PlanStep[];
		}[]
	>([]);
	const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
	const [pendingCommands, setPendingCommands] = useState<BoardCommand[]>([]);
	const [loading, setLoading] = useState(false);
	const [loadingPhase, setLoadingPhase] = useState<LoadingPhase>("initializing");
	const [loadingStartTime, setLoadingStartTime] = useState<number | null>(null);
	const [elapsedSeconds, setElapsedSeconds] = useState(0);
	const [currentToolCall, setCurrentToolCall] = useState<string | null>(null);
	const [tokenCount, setTokenCount] = useState(0);
	const [planSteps, setPlanSteps] = useState<PlanStep[]>([]);
	const [selectedModelId, setSelectedModelId] = useState<string | undefined>(
		undefined,
	);
	const [autocompleteEnabled, setAutocompleteEnabled] = useState(false);
	const backend = useBackend();

	// Track elapsed time during loading
	useEffect(() => {
		if (loading && loadingStartTime) {
			const interval = setInterval(() => {
				setElapsedSeconds(Math.floor((Date.now() - loadingStartTime) / 1000));
			}, 1000);
			return () => clearInterval(interval);
		}
		setElapsedSeconds(0);
	}, [loading, loadingStartTime]);

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

	const scrollToBottom = useCallback((force = false) => {
		if (userScrolledUp && !force) return;
		const container = scrollContainerRef.current;
		if (!container) return;

		// Smooth scroll to bottom
		container.scrollTo({
			top: container.scrollHeight,
			behavior: force ? "smooth" : "auto",
		});
		if (force) setUserScrolledUp(false);
	}, [userScrolledUp]);

	// Handle scroll events to detect when user scrolls up
	const handleScroll = useCallback(() => {
		const container = scrollContainerRef.current;
		if (!container) return;

		const { scrollTop, scrollHeight, clientHeight } = container;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
		const isAtBottom = distanceFromBottom < 30;

		setUserScrolledUp(!isAtBottom);
	}, []);

	// Auto-scroll on new messages (only if user hasn't scrolled up)
	useEffect(() => {
		if (!userScrolledUp) {
			// Immediate scroll for instant feedback
			scrollToBottom();
			// Delayed scroll to catch any layout shifts
			const timeout = setTimeout(() => scrollToBottom(), 100);
			return () => clearTimeout(timeout);
		}
	}, [messages, suggestions, loading, userScrolledUp, scrollToBottom]);

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
		setLoadingPhase("initializing");
		setLoadingStartTime(Date.now());
		setTokenCount(0);
		setSuggestions([]);
		setPendingCommands([]);
		setPlanSteps([]);
		setUserScrolledUp(false);

		try {
			if (!board) return;

			let currentMessageContent = "";
			setMessages((prev) => [...prev, { role: "assistant", content: "" }]);

			// Transition to analyzing phase after a brief moment
			setTimeout(() => setLoadingPhase("analyzing"), 300);

			const onToken = (token: string) => {
				console.log("[FlowCopilot] onToken received:", token.slice(0, 100));
				setTokenCount((prev) => prev + 1);

				// Check for plan step events
				const planStepMatch = token.match(/<plan_step>([\s\S]*?)<\/plan_step>/);
				if (planStepMatch) {
					console.log("[FlowCopilot] Plan step detected:", planStepMatch[1].slice(0, 100));
					try {
						const eventData = JSON.parse(planStepMatch[1]);
						if (eventData.PlanStep) {
							const step = eventData.PlanStep;
							// Update loading phase based on tool
							if (step.tool_name === "think") {
								setLoadingPhase("reasoning");
							} else if (step.tool_name?.includes("search") || step.tool_name?.includes("catalog")) {
								setLoadingPhase("searching");
							} else if (step.tool_name === "emit_commands") {
								setLoadingPhase("generating");
							}

							setPlanSteps((prev) => {
								const existingIndex = prev.findIndex((s) => s.id === step.id);
								if (existingIndex >= 0) {
									const updated = [...prev];
									updated[existingIndex] = step;
									return updated;
								}
								return [...prev, step];
							});
						}
					} catch {
						// Not valid JSON, ignore
					}
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
						const toolName = match[1];
						setCurrentToolCall(toolName);
						// Update phase based on tool type
						if (toolName.includes("search") || toolName.includes("catalog") || toolName.includes("filter")) {
							setLoadingPhase("searching");
						} else if (toolName === "think") {
							setLoadingPhase("reasoning");
						} else if (toolName === "emit_commands") {
							setLoadingPhase("generating");
						}
					}
					return;
				}
				if (token.includes("tool_result:")) {
					setCurrentToolCall(null);
					return;
				}

				// When we start receiving actual content, switch to generating phase
				if (currentMessageContent.length === 0 && token.trim()) {
					setLoadingPhase("generating");
				}

				currentMessageContent += token;
				console.log("[FlowCopilot] Adding to message, total length:", currentMessageContent.length);
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

			// Update the final message with agent type and plan steps
			setMessages((prev) => {
				const newMessages = [...prev];
				const lastMessage = newMessages[newMessages.length - 1];
				if (lastMessage && lastMessage.role === "assistant") {
					lastMessage.agentType = response.agent_type;
					lastMessage.planSteps = planSteps.filter(s => s.status === "Completed");
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

			setLoadingPhase("finalizing");
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
			setLoadingStartTime(null);
			setCurrentToolCall(null);
			setTokenCount(0);
		}
	};

	if (!isOpen) {
		return (
			<motion.div
				initial={{ scale: 0, opacity: 0 }}
				animate={{ scale: 1, opacity: 1 }}
				className="absolute top-20 right-4 z-40"
			>
				<div className="relative group">
					{/* Subtle ambient glow on hover only */}
					<div className="absolute -inset-2 bg-primary/30 rounded-full blur-xl opacity-0 group-hover:opacity-50 transition-all duration-300" />

					<Button
						className="relative rounded-full w-12 h-12 p-0 shadow-lg bg-background/90 backdrop-blur-sm border border-border/50 hover:border-primary/50 hover:bg-background hover:shadow-xl transition-all duration-200"
						onClick={() => setIsOpen(true)}
					>
						<SparklesIcon className="w-5 h-5 text-primary" />
					</Button>

					{autocompleteEnabled && (
						<motion.div
							initial={{ scale: 0 }}
							animate={{ scale: 1 }}
							className="absolute -bottom-0.5 -right-0.5 w-4 h-4 bg-green-500 border-2 border-background rounded-full flex items-center justify-center shadow-md"
						>
							<Wand2Icon className="w-2.5 h-2.5 text-white" />
						</motion.div>
					)}
				</div>
			</motion.div>
		);
	}

	return (
		<AnimatePresence>
			{isOpen && (
				<motion.div
					initial={{ opacity: 0, y: -20, scale: 0.95 }}
					animate={{ opacity: 1, y: 0, scale: 1 }}
					exit={{ opacity: 0, y: -20, scale: 0.95 }}
					transition={{ type: "spring", stiffness: 400, damping: 30 }}
					className="absolute top-20 right-4 z-40"
					style={{ width: 420, height: 560 }}
				>
					{/* Subtle pulsating glow */}
					{loading && (
						<motion.div
							className="absolute -inset-1 rounded-[28px] pointer-events-none bg-primary/20"
							style={{ filter: "blur(16px)" }}
							animate={{ opacity: [0.3, 0.6, 0.3], scale: [1, 1.02, 1] }}
							transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
						/>
					)}

					{/* Main container */}
					<div className="absolute inset-0 bg-background/95 backdrop-blur-xl rounded-3xl border border-border/40 flex flex-col overflow-hidden shadow-2xl">
						{/* Enhanced Header */}
						<div className="relative overflow-hidden">
							{/* Background gradient */}
							<div className="absolute inset-0 bg-linear-to-br from-primary/8 via-violet-500/5 to-pink-500/5" />

							{/* Animated mesh gradient overlay */}
							{loading && (
								<motion.div
									className="absolute inset-0 opacity-30"
									style={{
										background: "radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
									}}
									animate={{
										background: [
											"radial-gradient(circle at 30% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 70% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
											"radial-gradient(circle at 70% 50%, rgba(139, 92, 246, 0.3), transparent 50%), radial-gradient(circle at 30% 50%, rgba(236, 72, 153, 0.3), transparent 50%)",
										]
									}}
									transition={{ duration: 3, repeat: Infinity, repeatType: "reverse" }}
								/>
							)}

							<div className="relative p-4 pb-3">
								<div className="flex justify-between items-start">
									<div className="flex items-center gap-3">
										{/* Animated logo */}
										<div className="relative">
											<motion.div
												className="absolute inset-0 bg-linear-to-br from-primary to-violet-600 rounded-xl blur-lg opacity-50"
												animate={loading ? { scale: [1, 1.2, 1], opacity: [0.5, 0.8, 0.5] } : {}}
												transition={{ duration: 2, repeat: Infinity }}
											/>
											<div className="relative p-2.5 bg-linear-to-br from-primary via-violet-600 to-pink-600 rounded-xl shadow-lg">
												<SparklesIcon className="w-5 h-5 text-white" />
											</div>
										</div>
										<div>
											<h3 className="font-bold text-base tracking-tight">FlowPilot</h3>
											<div className="flex items-center gap-2">
												{loading ? (
													<StatusPill phase={loadingPhase} elapsed={elapsedSeconds} />
												) : (
													<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
														<span className="relative flex h-2 w-2">
															<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
															<span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
														</span>
														Ready to help
													</div>
												)}
											</div>
										</div>
									</div>

									<div className="flex items-center gap-1">
										<Tooltip>
											<TooltipTrigger asChild>
												<Button
													variant={autocompleteEnabled ? "default" : "ghost"}
													size="icon"
													className={`h-8 w-8 rounded-lg transition-all duration-200 ${
														autocompleteEnabled
															? "bg-primary/90 hover:bg-primary shadow-md shadow-primary/20"
															: "hover:bg-accent/50"
													}`}
													onClick={() => setAutocompleteEnabled(!autocompleteEnabled)}
												>
													<Wand2Icon className={`w-4 h-4 ${autocompleteEnabled ? "text-primary-foreground" : ""}`} />
												</Button>
											</TooltipTrigger>
											<TooltipContent side="bottom" className="text-xs">
												{autocompleteEnabled ? "Disable" : "Enable"} autocomplete
											</TooltipContent>
										</Tooltip>

										<Tooltip>
											<TooltipTrigger asChild>
												<Button
													variant="ghost"
													size="icon"
													className="h-8 w-8 rounded-lg hover:bg-accent/50"
													onClick={handleNewChat}
												>
													<SquarePenIcon className="w-4 h-4" />
												</Button>
											</TooltipTrigger>
											<TooltipContent side="bottom" className="text-xs">
												New chat
											</TooltipContent>
										</Tooltip>

										<Tooltip>
											<TooltipTrigger asChild>
												<Button
													variant="ghost"
													size="icon"
													className="h-8 w-8 rounded-lg hover:bg-destructive/10 hover:text-destructive"
													onClick={() => setIsOpen(false)}
												>
													<XIcon className="w-4 h-4" />
												</Button>
											</TooltipTrigger>
											<TooltipContent side="bottom" className="text-xs">
												Close
											</TooltipContent>
										</Tooltip>
									</div>
								</div>
							</div>

							{/* Model selector row */}
							<div className="relative px-4 pb-3">
								<Select value={selectedModelId} onValueChange={setSelectedModelId}>
									<SelectTrigger className="h-9 text-xs bg-background/60 backdrop-blur-sm border-border/30 hover:border-primary/30 transition-all duration-200 rounded-xl focus:ring-2 focus:ring-primary/20">
										<div className="flex items-center gap-2">
											<BotIcon className="w-3.5 h-3.5 text-muted-foreground" />
											<SelectValue placeholder="Select Model" />
										</div>
									</SelectTrigger>
									<SelectContent className="rounded-xl">
										{models.map((model) => (
											<SelectItem key={model.id} value={model.id} className="text-xs rounded-lg">
												{model.meta?.en?.name || model.id}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>

							{/* Progress bar when loading */}
							{loading && (
								<motion.div
									className="absolute bottom-0 left-0 right-0 h-0.5 bg-muted/30"
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
								>
									<motion.div
										className="h-full bg-linear-to-r from-primary via-violet-500 to-pink-500"
										initial={{ width: "0%" }}
										animate={{ width: "100%" }}
										transition={{ duration: 30, ease: "linear" }}
									/>
								</motion.div>
							)}
						</div>

						{/* Chat Area */}
						<ScrollArea
							className="flex-1 p-4 flex flex-col max-h-full overflow-auto"
							viewportRef={scrollContainerRef}
							onScroll={handleScroll}
						>
							<div className="space-y-4 min-w-0">
								{messages.length === 0 && (
									<motion.div
										initial={{ opacity: 0, y: 10 }}
										animate={{ opacity: 1, y: 0 }}
										className="text-center py-12 space-y-4"
									>
										<div className="relative inline-block">
											<div className="absolute inset-0 bg-linear-to-br from-primary/30 via-violet-500/20 to-pink-500/30 blur-3xl rounded-full scale-150" />
											<motion.div
												animate={{ rotate: [0, 5, -5, 0] }}
												transition={{ duration: 4, repeat: Infinity, ease: "easeInOut" }}
											>
												<SparklesIcon className="w-14 h-14 mx-auto relative text-primary/50" />
											</motion.div>
										</div>
										<div className="space-y-1.5">
											<p className="text-base font-semibold text-foreground">
												How can I help you today?
											</p>
											<p className="text-sm text-muted-foreground max-w-[280px] mx-auto">
												Describe what you want to build or modify in your flow
											</p>
										</div>
										<div className="flex flex-wrap gap-2 justify-center pt-2">
											{["Add a node", "Connect components", "Explain my flow"].map((suggestion, i) => (
												<motion.button
													key={suggestion}
													initial={{ opacity: 0, y: 5 }}
													animate={{ opacity: 1, y: 0 }}
													transition={{ delay: 0.2 + i * 0.1 }}
													onClick={() => setInput(suggestion)}
													className="px-3 py-1.5 text-xs font-medium text-muted-foreground bg-muted/50 hover:bg-muted border border-border/50 hover:border-primary/30 rounded-full transition-all duration-200 hover:text-foreground"
												>
													{suggestion}
												</motion.button>
											))}
										</div>
									</motion.div>
								)}

								{messages.map((m, i) => (
									<motion.div
										initial={{ opacity: 0, y: 10 }}
										animate={{ opacity: 1, y: 0 }}
										transition={{ delay: i === messages.length - 1 ? 0.1 : 0 }}
										key={i}
										className={`flex min-w-0 ${m.role === "user" ? "justify-end" : "justify-start"}`}
									>
										<div
											className={`p-3.5 rounded-2xl text-sm wrap-break-word overflow-hidden transition-all duration-200 min-w-0 ${
												m.role === "user"
													? "bg-muted/60 text-foreground rounded-br-md max-w-[85%] border border-border/40"
													: "bg-muted/40 backdrop-blur-sm rounded-bl-md max-w-full border border-border/30"
											}`}
										>
											{m.role === "assistant" && m.agentType && (
												<div className="flex items-center gap-2 mb-2.5 pb-2 border-b border-border/30">
													<div className={`p-1 rounded-md ${m.agentType === "Explain" ? "bg-blue-500/15" : "bg-amber-500/15"}`}>
														{m.agentType === "Explain" ? (
															<BrainCircuitIcon className="w-3 h-3 text-blue-500" />
														) : (
															<EditIcon className="w-3 h-3 text-amber-500" />
														)}
													</div>
													<span className={`text-xs font-medium ${m.agentType === "Explain" ? "text-blue-500" : "text-amber-500"}`}>
														{m.agentType === "Explain" ? "Explain Mode" : "Edit Mode"}
													</span>
												</div>
											)}
											{m.content ? (
												<>
													<MessageContent
														content={m.content}
														onFocusNode={onFocusNode}
														board={board}
													/>
													{/* Show plan steps at bottom of completed messages */}
													{m.planSteps && m.planSteps.length > 0 && (
														<Collapsible className="mt-3 pt-3 border-t border-border/30">
															<CollapsibleTrigger className="flex items-center gap-1.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors w-full">
																<ChevronDown className="w-3 h-3 transition-transform duration-200 group-data-[state=open]:rotate-180" />
																<ListChecksIcon className="w-3 h-3" />
																<span>{m.planSteps.length} steps completed</span>
															</CollapsibleTrigger>
															<CollapsibleContent>
																<div className="mt-2 space-y-1">
																	{m.planSteps.map((step, idx) => (
																		<div key={idx} className="flex items-center gap-2 text-[11px] text-muted-foreground">
																			<CheckCircle2Icon className="w-3 h-3 text-green-500 shrink-0" />
																			<span className="truncate">{step.description}</span>
																		</div>
																	))}
																</div>
															</CollapsibleContent>
														</Collapsible>
													)}
												</>
											) : (
												<div className="space-y-3">
													{/* Separate thinking view */}
													{planSteps.some(s => s.tool_name === "think" && s.status === "InProgress") ? (
														<div className="space-y-2">
															<div className="flex items-center gap-2">
																<BrainCircuitIcon className="w-3.5 h-3.5 text-violet-500 animate-pulse" />
																<span className="text-xs font-medium text-violet-500">Reasoning...</span>
															</div>
															<div className="text-[11px] text-muted-foreground font-mono bg-violet-500/5 rounded-lg p-2.5 max-h-24 overflow-y-auto border border-violet-500/20">
																{planSteps.find(s => s.tool_name === "think")?.description}
																<span className="inline-block w-1.5 h-3 ml-0.5 bg-violet-500/60 animate-pulse rounded-sm" />
															</div>
														</div>
													) : (
														<div className="flex items-center gap-2.5">
															<div className="flex gap-1">
																{[0, 1, 2].map((j) => (
																	<motion.span
																		key={j}
																		className="w-1.5 h-1.5 bg-primary rounded-full"
																		animate={{ opacity: [0.3, 1, 0.3] }}
																		transition={{
																			duration: 1,
																			repeat: Infinity,
																			delay: j * 0.2,
																		}}
																	/>
																))}
															</div>
															<span className="text-xs text-muted-foreground">
																{currentToolCall ? `Using ${currentToolCall.replace(/_/g, " ")}...` : "Processing..."}
															</span>
														</div>
													)}
													{/* Show plan steps inline during generation */}
													{planSteps.length > 0 && !planSteps.some(s => s.tool_name === "think" && s.status === "InProgress") && (
														<PlanStepsView steps={planSteps} />
													)}
												</div>
											)}
											{/* Show executed commands */}
											{m.executedCommands && m.executedCommands.length > 0 && (
												<motion.div
													initial={{ opacity: 0, height: 0 }}
													animate={{ opacity: 1, height: "auto" }}
													className="mt-3 pt-3 border-t border-border/30"
												>
													<div className="flex items-center gap-1.5 mb-2 text-xs text-muted-foreground">
														<div className="p-0.5 bg-green-500/20 rounded">
															<CheckCircle2Icon className="w-3 h-3 text-green-500" />
														</div>
														<span className="font-medium">
															Applied {m.executedCommands.length} change{m.executedCommands.length > 1 ? "s" : ""}
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
												</motion.div>
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

						{/* Scroll to bottom indicator */}
						<AnimatePresence>
							{userScrolledUp && messages.length > 0 && (
								<motion.div
									initial={{ opacity: 0, y: 10 }}
									animate={{ opacity: 1, y: 0 }}
									exit={{ opacity: 0, y: 10 }}
									className="absolute bottom-20 left-1/2 -translate-x-1/2 z-10"
								>
									<Button
										size="sm"
										variant="secondary"
										className="rounded-full shadow-lg border border-border/50 gap-1.5 px-3 h-8 text-xs"
										onClick={() => scrollToBottom(true)}
									>
										<ArrowDownIcon className="w-3.5 h-3.5" />
										New messages
									</Button>
								</motion.div>
							)}
						</AnimatePresence>

						{/* Input Area */}
						<div className="p-4 bg-background/80 backdrop-blur-sm border-t border-border/50">
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
