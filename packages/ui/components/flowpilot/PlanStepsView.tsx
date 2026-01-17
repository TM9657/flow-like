"use client";

import { motion } from "framer-motion";
import {
	BrainCircuitIcon,
	CheckCircle2Icon,
	ChevronDown,
	CircleDashedIcon,
	LoaderCircleIcon,
	SearchIcon,
	SparklesIcon,
	TargetIcon,
	WrenchIcon,
	XCircleIcon,
	ZapIcon,
} from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";

import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../ui/collapsible";

import type { UnifiedPlanStep } from "./types";

interface PlanStepsViewProps {
	steps: UnifiedPlanStep[];
	/** Whether the copilot is currently processing (affects display behavior) */
	loading?: boolean;
	/** Compact mode for smaller UI contexts */
	compact?: boolean;
}

function getStatusIcon(status: string) {
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
			return (
				<CircleDashedIcon className="w-3.5 h-3.5 text-muted-foreground/50" />
			);
	}
}

function getToolIcon(toolName?: string) {
	if (!toolName) return <ZapIcon className="w-3 h-3" />;
	switch (toolName) {
		case "catalog_search":
		case "search_by_pin":
		case "filter_category":
		case "get_component_schema":
		case "get_style_examples":
			return <SearchIcon className="w-3 h-3" />;
		case "think":
		case "analyze":
			return <BrainCircuitIcon className="w-3 h-3" />;
		case "focus_node":
			return <TargetIcon className="w-3 h-3" />;
		case "emit_commands":
		case "emit_surface":
		case "modify_component":
			return <SparklesIcon className="w-3 h-3" />;
		default:
			return <WrenchIcon className="w-3 h-3" />;
	}
}

function getToolLabel(toolName?: string) {
	if (!toolName) return "Processing";
	switch (toolName) {
		case "catalog_search":
			return "Searching catalog";
		case "search_by_pin":
			return "Finding pins";
		case "filter_category":
			return "Filtering";
		case "think":
			return "Reasoning";
		case "analyze":
			return "Analyzing";
		case "focus_node":
			return "Focusing";
		case "emit_commands":
			return "Building flow";
		case "emit_surface":
			return "Generating UI";
		case "get_component_schema":
			return "Looking up schema";
		case "get_style_examples":
			return "Fetching styles";
		case "modify_component":
			return "Modifying component";
		default:
			return toolName.replace(/_/g, " ");
	}
}

export const PlanStepsView = memo(function PlanStepsView({
	steps,
	loading = false,
	compact = false,
}: PlanStepsViewProps) {
	const [expanded, setExpanded] = useState(false);
	const toggleExpanded = useCallback((open: boolean) => setExpanded(open), []);

	// Memoize computed values
	const {
		completedCount,
		progress,
		currentStep,
		historySteps,
		inProgressSteps,
	} = useMemo(() => {
		const completed = steps.filter((s) => s.status === "Completed").length;
		const inProgress = steps.filter((s) => s.status === "InProgress");
		const current =
			steps.findLast((s) => s.status === "InProgress") ||
			steps[steps.length - 1];
		return {
			completedCount: completed,
			progress: steps.length > 0 ? (completed / steps.length) * 100 : 0,
			currentStep: current,
			historySteps: steps.filter(
				(s) => s.id !== current?.id && s.status === "Completed",
			),
			inProgressSteps: inProgress,
		};
	}, [steps]);

	if (steps.length === 0) return null;

	// Compact mode - shows current activity and collapsible history
	if (compact) {
		const hasCompletedSteps = completedCount > 0;
		const hasInProgressSteps = inProgressSteps.length > 0;

		// During loading, show current activity
		if (loading && currentStep) {
			return (
				<div className="border-t border-border/30 bg-muted/20 px-3 py-2 space-y-2">
					{/* Current step with prominent display */}
					<motion.div
						key={currentStep.id}
						initial={{ opacity: 0, y: -5 }}
						animate={{ opacity: 1, y: 0 }}
						className={`flex items-start gap-2 p-2 rounded-lg ${
							currentStep.status === "InProgress"
								? "bg-primary/5 border border-primary/20"
								: "bg-muted/30"
						}`}
					>
						{getStatusIcon(currentStep.status)}
						<div className="flex-1 min-w-0">
							<div className="flex items-center gap-2">
								{currentStep.tool_name && (
									<span className="text-[10px] font-medium text-primary flex items-center gap-1">
										{getToolIcon(currentStep.tool_name)}
										{getToolLabel(currentStep.tool_name)}
									</span>
								)}
							</div>
							{currentStep.tool_name === "think" ||
							currentStep.tool_name === "analyze" ? (
								<p className="text-[10px] text-muted-foreground mt-1 whitespace-pre-wrap line-clamp-3">
									{currentStep.description}
								</p>
							) : (
								<p className="text-[10px] text-muted-foreground mt-0.5 truncate">
									{currentStep.description}
								</p>
							)}
						</div>
					</motion.div>

					{/* Completed steps summary */}
					{hasCompletedSteps && (
						<Collapsible open={expanded} onOpenChange={toggleExpanded}>
							<CollapsibleTrigger className="flex items-center justify-between w-full py-1 hover:bg-muted/30 rounded transition-colors">
								<div className="flex items-center gap-2">
									<CheckCircle2Icon className="h-3 w-3 text-green-500" />
									<span className="text-[10px] text-muted-foreground">
										{completedCount} step{completedCount !== 1 ? "s" : ""}{" "}
										completed
									</span>
								</div>
								<ChevronDown
									className={`h-3 w-3 text-muted-foreground transition-transform ${expanded ? "rotate-180" : ""}`}
								/>
							</CollapsibleTrigger>
							<CollapsibleContent>
								<div className="pt-1 space-y-1">
									{historySteps.map((step) => (
										<div
											key={step.id}
											className="flex items-start gap-2 text-[10px]"
										>
											{getStatusIcon(step.status)}
											<span className="text-muted-foreground leading-tight truncate">
												{step.description}
											</span>
										</div>
									))}
								</div>
							</CollapsibleContent>
						</Collapsible>
					)}
				</div>
			);
		}

		// Not loading - show summary only if we have completed steps
		if (!hasCompletedSteps) return null;

		return (
			<Collapsible
				open={expanded}
				onOpenChange={toggleExpanded}
				className="border-t border-border/30 bg-muted/20"
			>
				<CollapsibleTrigger className="flex items-center justify-between w-full px-3 py-1.5 hover:bg-muted/30 transition-colors">
					<div className="flex items-center gap-2">
						<CheckCircle2Icon className="h-3 w-3 text-green-500" />
						<span className="text-[10px] text-muted-foreground">
							{completedCount} step{completedCount !== 1 ? "s" : ""} completed
						</span>
					</div>
					<ChevronDown
						className={`h-3 w-3 text-muted-foreground transition-transform ${expanded ? "rotate-180" : ""}`}
					/>
				</CollapsibleTrigger>
				<CollapsibleContent>
					<div className="px-3 pb-2 space-y-1">
						{steps.map((step) => (
							<div key={step.id} className="flex items-start gap-2 text-[10px]">
								{getStatusIcon(step.status)}
								<span className="text-muted-foreground leading-tight">
									{step.description}
								</span>
							</div>
						))}
					</div>
				</CollapsibleContent>
			</Collapsible>
		);
	}

	// Full mode with progress bar
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
							transition={{
								duration: 3,
								repeat: Number.POSITIVE_INFINITY,
								ease: "linear",
							}}
						/>
					)}
					<div className="relative p-3 flex items-start gap-2.5 max-w-full overflow-hidden">
						<div className="shrink-0 mt-0.5">
							{getStatusIcon(currentStep.status)}
						</div>
						<div className="flex-1 min-w-0 overflow-hidden">
							{currentStep.tool_name === "think" ? (
								<Collapsible defaultOpen={currentStep.status === "InProgress"}>
									<CollapsibleTrigger className="flex items-center gap-2 w-full text-left group">
										<span className="text-xs font-medium text-foreground">
											Reasoning
										</span>
										<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180 text-muted-foreground shrink-0" />
									</CollapsibleTrigger>
									<CollapsibleContent>
										<div className="mt-2 text-[11px] text-muted-foreground whitespace-pre-wrap font-mono bg-background/60 rounded-lg p-2.5 max-h-28 overflow-y-auto border border-border/30">
											{currentStep.description}
										</div>
									</CollapsibleContent>
								</Collapsible>
							) : (
								<div className="flex items-center gap-2 min-w-0">
									<span className="text-xs font-medium text-foreground truncate">
										{currentStep.description}
									</span>
								</div>
							)}
						</div>
						{currentStep.tool_name && (
							<div className="shrink-0 flex items-center gap-1 text-[10px] text-muted-foreground px-2 py-1 rounded-lg bg-background/60 border border-border/30">
								{getToolIcon(currentStep.tool_name)}
								<span className="hidden sm:inline">
									{getToolLabel(currentStep.tool_name)}
								</span>
							</div>
						)}
					</div>
				</motion.div>
			)}

			{/* History steps collapsible */}
			{historySteps.length > 0 && (
				<Collapsible open={expanded} onOpenChange={toggleExpanded}>
					<CollapsibleTrigger className="flex items-center gap-2 w-full text-left px-1 py-0.5 hover:bg-muted/30 rounded-lg transition-colors group">
						<span className="text-[10px] text-muted-foreground">
							{historySteps.length} previous step
							{historySteps.length !== 1 ? "s" : ""}
						</span>
						<ChevronDown className="w-3 h-3 ml-auto transition-transform duration-200 group-data-[state=open]:rotate-180 text-muted-foreground" />
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="space-y-1.5 mt-2 pl-1">
							{historySteps.map((step) => (
								<div
									key={step.id}
									className="flex items-start gap-2 text-[10px] text-muted-foreground"
								>
									{getStatusIcon(step.status)}
									<span className="truncate">{step.description}</span>
								</div>
							))}
						</div>
					</CollapsibleContent>
				</Collapsible>
			)}
		</motion.div>
	);
});
