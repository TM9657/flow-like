"use client";

import { motion } from "framer-motion";
import {
	BrainCircuitIcon,
	CheckCircle2Icon,
	CpuIcon,
	SearchIcon,
	SparklesIcon,
} from "lucide-react";
import { memo, useMemo } from "react";

import type { LoadingPhase } from "./types";

export const LOADING_PHASE_CONFIG: Record<
	LoadingPhase,
	{ label: string; color: string }
> = {
	idle: { label: "Ready", color: "text-muted-foreground" },
	initializing: { label: "Starting up...", color: "text-blue-500" },
	analyzing: { label: "Analyzing...", color: "text-violet-500" },
	searching: { label: "Searching...", color: "text-cyan-500" },
	reasoning: { label: "Thinking...", color: "text-amber-500" },
	generating: { label: "Generating...", color: "text-pink-500" },
	finalizing: { label: "Finalizing...", color: "text-green-500" },
};

function getPhaseIcon(phase: LoadingPhase) {
	switch (phase) {
		case "reasoning":
			return <BrainCircuitIcon className="w-3.5 h-3.5" />;
		case "generating":
			return <SparklesIcon className="w-3.5 h-3.5" />;
		case "searching":
		case "analyzing":
			return <SearchIcon className="w-3.5 h-3.5" />;
		case "finalizing":
			return <CheckCircle2Icon className="w-3.5 h-3.5" />;
		default:
			return <CpuIcon className="w-3.5 h-3.5" />;
	}
}

interface StatusPillProps {
	phase: LoadingPhase;
	elapsed: number;
	compact?: boolean;
}

export const StatusPill = memo(function StatusPill({
	phase,
	elapsed,
	compact = false,
}: StatusPillProps) {
	const phaseInfo = useMemo(() => LOADING_PHASE_CONFIG[phase], [phase]);

	if (compact) {
		return (
			<motion.div
				initial={{ opacity: 0, scale: 0.9 }}
				animate={{ opacity: 1, scale: 1 }}
				className={`inline-flex items-center gap-1 text-xs ${phaseInfo.color}`}
			>
				<motion.div
					animate={{ rotate: phase === "reasoning" ? 360 : 0 }}
					transition={{
						duration: 2,
						repeat: phase === "reasoning" ? Number.POSITIVE_INFINITY : 0,
						ease: "linear",
					}}
				>
					{getPhaseIcon(phase)}
				</motion.div>
				<span>{phaseInfo.label}</span>
				{elapsed > 0 && (
					<span className="text-muted-foreground/60 tabular-nums">
						{elapsed}s
					</span>
				)}
			</motion.div>
		);
	}

	return (
		<motion.div
			initial={{ opacity: 0, scale: 0.9 }}
			animate={{ opacity: 1, scale: 1 }}
			className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-background/80 backdrop-blur-sm border border-border/50 ${phaseInfo.color}`}
		>
			<motion.div
				animate={{ rotate: phase === "reasoning" ? 360 : 0 }}
				transition={{
					duration: 2,
					repeat: phase === "reasoning" ? Number.POSITIVE_INFINITY : 0,
					ease: "linear",
				}}
			>
				{getPhaseIcon(phase)}
			</motion.div>
			<span>{phaseInfo.label}</span>
			{elapsed > 0 && (
				<span className="text-muted-foreground/60 tabular-nums">
					{elapsed}s
				</span>
			)}
		</motion.div>
	);
});
