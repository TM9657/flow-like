"use client";

import {
	CheckCircle2,
	ChevronDown,
	ChevronRight,
	Circle,
	Clock,
	ListTodo,
	Loader2,
	XCircle,
} from "lucide-react";
import { useEffect, useState } from "react";
import { formatDuration } from "../../../lib/date";
import type { IPlanStep, PlanStepStatus } from "./chat-db";
import { ReasoningViewer } from "./reasoning-viewer";

interface PlanStepsProps {
	steps: IPlanStep[];
	currentStepId?: string;
}

function getStatusIcon(status: PlanStepStatus) {
	switch (status) {
		case "planned":
			return <Circle className="w-4 h-4 text-muted-foreground" />;
		case "progress":
			return <Loader2 className="w-4 h-4 text-primary animate-spin" />;
		case "done":
			return <CheckCircle2 className="w-4 h-4 text-green-500" />;
		case "failed":
			return <XCircle className="w-4 h-4 text-red-500" />;
	}
}

function getStatusColor(status: PlanStepStatus, isActive: boolean) {
	if (isActive) {
		return "border-primary/70 bg-primary/20 shadow-sm";
	}
	switch (status) {
		case "planned":
			return "border-muted-foreground/30 bg-muted/20";
		case "progress":
			return "border-primary/50 bg-primary/10";
		case "done":
			return "border-green-500/50 bg-green-500/10";
		case "failed":
			return "border-red-500/50 bg-red-500/10";
	}
}

function getStatusBadge(status: PlanStepStatus) {
	switch (status) {
		case "planned":
			return (
				<span className="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground">
					Planned
				</span>
			);
		case "progress":
			return (
				<span className="text-xs px-2 py-0.5 rounded-full bg-primary/20 text-primary font-medium">
					In Progress
				</span>
			);
		case "done":
			return (
				<span className="text-xs px-2 py-0.5 rounded-full bg-green-500/20 text-green-600 dark:text-green-400">
					Completed
				</span>
			);
		case "failed":
			return (
				<span className="text-xs px-2 py-0.5 rounded-full bg-red-500/20 text-red-600 dark:text-red-400">
					Failed
				</span>
			);
	}
}

export function PlanSteps({ steps, currentStepId }: PlanStepsProps) {
	// Keep current step expanded, but allow all steps to be toggled
	const [expandedSteps, setExpandedSteps] = useState<Set<string>>(
		new Set(currentStepId ? [currentStepId] : []),
	);

	// Auto-expand current step when it changes
	useEffect(() => {
		if (currentStepId) {
			setExpandedSteps((prev) => {
				if (!prev.has(currentStepId)) {
					const next = new Set(prev);
					next.add(currentStepId);
					return next;
				}
				return prev;
			});
		}
	}, [currentStepId]);

	if (!steps || steps.length === 0) {
		return null;
	}

	const toggleStep = (stepId: string) => {
		setExpandedSteps((prev) => {
			const next = new Set(prev);
			if (next.has(stepId)) {
				next.delete(stepId);
			} else {
				next.add(stepId);
			}
			return next;
		});
	};

	const completedCount = steps.filter((s) => s.status === "done").length;
	const failedCount = steps.filter((s) => s.status === "failed").length;

	return (
		<div className="my-3 border rounded-xl overflow-hidden bg-linear-to-br from-muted/30 to-muted/10">
			<div className="flex items-center gap-3 p-4 border-b bg-muted/50 backdrop-blur-sm">
				<div className="flex items-center gap-2 flex-1">
					<ListTodo className="w-5 h-5 text-primary" />
					<span className="text-sm font-semibold">Execution Plan</span>
				</div>
				<div className="flex items-center gap-3">
					{failedCount > 0 && (
						<span className="text-xs px-2 py-1 rounded-full bg-red-500/20 text-red-600 dark:text-red-400 font-medium">
							{failedCount} failed
						</span>
					)}
					<span className="text-xs px-2 py-1 rounded-full bg-green-500/20 text-green-600 dark:text-green-400 font-medium">
						{completedCount} / {steps.length}
					</span>
				</div>
			</div>
			<div className="p-3 space-y-2">
				{steps.map((step, index) => {
					const isExpanded = expandedSteps.has(step.id);
					const isActive = currentStepId === step.id;
					const hasReasoning = step.reasoning && step.reasoning.trim() !== "";
					const duration =
						step.startTime && step.endTime
							? formatDuration((step.endTime - step.startTime) * 1000)
							: null;

					return (
						<div
							key={step.id}
							className={`rounded-lg border transition-all ${getStatusColor(
								step.status,
								isActive,
							)}`}
						>
							<button
								onClick={() => hasReasoning && toggleStep(step.id)}
								className={`w-full flex items-start gap-3 p-3 text-left transition-colors ${
									hasReasoning
										? "hover:bg-muted/30 cursor-pointer"
										: "cursor-default"
								}`}
							>
								<div className="shrink-0 mt-0.5">
									{getStatusIcon(step.status)}
								</div>
								<div className="flex-1 min-w-0">
									<div className="flex items-start justify-between gap-2 mb-1">
										<div className="flex-1 min-w-0">
											<div className="flex items-center gap-2 mb-1">
												<h4 className="text-sm font-medium">{step.title}</h4>
												{isActive && (
													<span className="px-2 py-0.5 text-[10px] rounded-full bg-primary/30 text-primary font-medium animate-pulse">
														ACTIVE
													</span>
												)}
											</div>
											{step.description && (
												<p className="text-xs text-muted-foreground">
													{step.description}
												</p>
											)}
										</div>
										<div className="flex items-center gap-2 shrink-0">
											<span className="text-[10px] text-muted-foreground font-mono">
												#{index + 1}
											</span>
											{getStatusBadge(step.status)}
										</div>
									</div>
									{duration && (
										<div className="flex items-center gap-1 mt-1">
											<Clock className="w-3 h-3 text-muted-foreground" />
											<span className="text-xs text-muted-foreground">
												{duration}
											</span>
										</div>
									)}
								</div>
								{hasReasoning && (
									<div className="shrink-0 mt-1">
										{isExpanded ? (
											<ChevronDown className="w-4 h-4 text-muted-foreground" />
										) : (
											<ChevronRight className="w-4 h-4 text-muted-foreground" />
										)}
									</div>
								)}
							</button>
							{isExpanded && hasReasoning && (
								<div className="px-3 pb-3 pt-0">
									<ReasoningViewer
										reasoning={step.reasoning!}
										defaultExpanded={true}
										compact={true}
									/>
								</div>
							)}
						</div>
					);
				})}
			</div>
		</div>
	);
}
