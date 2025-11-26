"use client";

import { motion } from "framer-motion";
import {
	BrainCircuitIcon,
	CheckCircle2Icon,
	CpuIcon,
	SearchIcon,
	SparklesIcon,
} from "lucide-react";

import type { LoadingPhase, LoadingPhaseInfo } from "./types";

export const LOADING_PHASES: Record<LoadingPhase, LoadingPhaseInfo> = {
	initializing: {
		label: "Initializing...",
		icon: <CpuIcon className="w-3.5 h-3.5" />,
		color: "text-blue-500",
	},
	analyzing: {
		label: "Analyzing your flow...",
		icon: <SearchIcon className="w-3.5 h-3.5" />,
		color: "text-violet-500",
	},
	searching: {
		label: "Searching catalog...",
		icon: <SearchIcon className="w-3.5 h-3.5" />,
		color: "text-cyan-500",
	},
	reasoning: {
		label: "Thinking...",
		icon: <BrainCircuitIcon className="w-3.5 h-3.5" />,
		color: "text-amber-500",
	},
	generating: {
		label: "Generating response...",
		icon: <SparklesIcon className="w-3.5 h-3.5" />,
		color: "text-pink-500",
	},
	finalizing: {
		label: "Finalizing...",
		icon: <CheckCircle2Icon className="w-3.5 h-3.5" />,
		color: "text-green-500",
	},
};

interface StatusPillProps {
	phase: LoadingPhase;
	elapsed: number;
}

export function StatusPill({ phase, elapsed }: StatusPillProps) {
	const phaseInfo = LOADING_PHASES[phase];
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
				{phaseInfo.icon}
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
