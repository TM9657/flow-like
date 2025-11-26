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
} from "../../ui/collapsible";

import type { PlanStep } from "../../../lib/schema/flow/copilot";

interface PlanStepsViewProps {
	steps: PlanStep[];
}

function getStatusIcon(status: PlanStep["status"]) {
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
}

function getToolLabel(toolName?: string) {
	if (!toolName) return "Processing";
	switch (toolName) {
		case "catalog_search":
			return "Searching";
		case "search_by_pin":
			return "Finding pins";
		case "filter_category":
			return "Filtering";
		case "think":
			return "Reasoning";
		case "focus_node":
			return "Focusing";
		case "emit_commands":
			return "Building";
		default:
			return toolName.replace(/_/g, " ");
	}
}

export const PlanStepsView = memo(function PlanStepsView({
	steps,
}: PlanStepsViewProps) {
	const [expanded, setExpanded] = useState(false);
	const toggleExpanded = useCallback((open: boolean) => setExpanded(open), []);

	// Memoize computed values
	const { completedCount, progress, currentStep, historySteps } =
		useMemo(() => {
			const completed = steps.filter((s) => s.status === "Completed").length;
			const current =
				steps.findLast((s) => s.status === "InProgress") ||
				steps[steps.length - 1];
			return {
				completedCount: completed,
				progress: steps.length > 0 ? (completed / steps.length) * 100 : 0,
				currentStep: current,
				historySteps: steps.filter((s) => s.id !== current?.id),
			};
		}, [steps]);

	if (steps.length === 0) return null;

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
											{currentStep.status === "InProgress" && (
												<span className="inline-block w-1.5 h-3 ml-0.5 bg-primary/60 animate-pulse rounded-sm" />
											)}
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

			{/* Expandable history */}
			{steps.length > 1 && (
				<Collapsible open={expanded} onOpenChange={toggleExpanded}>
					<CollapsibleTrigger className="flex items-center gap-2 text-[11px] text-muted-foreground hover:text-foreground transition-colors w-full py-1">
						<ChevronDown
							className={`w-3 h-3 transition-transform duration-200 ${expanded ? "rotate-180" : ""}`}
						/>
						<span>
							{expanded ? "Hide" : "Show"} {steps.length - 1} previous step
							{steps.length > 2 ? "s" : ""}
						</span>
					</CollapsibleTrigger>
					<CollapsibleContent>
						<div className="mt-2 space-y-1.5 pl-1">
							{historySteps.map((step) => (
								<motion.div
									key={step.id}
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									className="flex items-center gap-2 text-[11px] text-muted-foreground py-1 max-w-full"
								>
									{getStatusIcon(step.status)}
									<span className="truncate flex-1 min-w-0">
										{step.description}
									</span>
								</motion.div>
							))}
						</div>
					</CollapsibleContent>
				</Collapsible>
			)}
		</motion.div>
	);
});
