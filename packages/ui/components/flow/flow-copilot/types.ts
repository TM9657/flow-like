import type React from "react";
import type { IBoard, ILogMetadata } from "../../../lib";
import type {
	BoardCommand,
	PlanStep,
	Suggestion,
} from "../../../lib/schema/flow/copilot";

export type LoadingPhase =
	| "initializing"
	| "analyzing"
	| "searching"
	| "reasoning"
	| "generating"
	| "finalizing";

export interface LoadingPhaseInfo {
	label: string;
	icon: React.ReactNode;
	color: string;
}

export type Mode = "chat" | "autocomplete";

export interface CopilotMessage {
	role: "user" | "assistant";
	content: string;
	agentType?: "Explain" | "Edit";
	executedCommands?: BoardCommand[];
	planSteps?: PlanStep[];
}

export interface FlowCopilotProps {
	board: IBoard | null | undefined;
	selectedNodeIds: string[];
	onAcceptSuggestion: (suggestion: Suggestion) => void;
	onExecuteCommands?: (commands: BoardCommand[]) => void;
	onGhostNodesChange?: (suggestions: Suggestion[]) => void;
	onClearRunContext?: () => void;
	mode?: Mode;
	embedded?: boolean;
	runContext?: ILogMetadata;
	onFocusNode?: (nodeId: string) => void;
}

export const LOADING_PHASES: Record<LoadingPhase, LoadingPhaseInfo> = {
	initializing: {
		label: "Starting up",
		icon: null,
		color: "text-muted-foreground",
	},
	analyzing: {
		label: "Analyzing flow",
		icon: null,
		color: "text-blue-500",
	},
	searching: {
		label: "Searching catalog",
		icon: null,
		color: "text-violet-500",
	},
	reasoning: {
		label: "Reasoning",
		icon: null,
		color: "text-amber-500",
	},
	generating: {
		label: "Generating",
		icon: null,
		color: "text-emerald-500",
	},
	finalizing: {
		label: "Finalizing",
		icon: null,
		color: "text-primary",
	},
};
