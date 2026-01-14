import type React from "react";
import type { IBoard } from "../../lib";
import type { SurfaceComponent } from "../a2ui/types";
import type {
	BoardCommand,
	PlanStep,
	Suggestion,
} from "../../lib/schema/flow/copilot";
import type { A2UIPlanStep } from "../../lib/schema/a2ui/copilot";

/**
 * Agent mode determines what the copilot operates on:
 * - "board": Flow board operations (adding nodes, connections, etc.)
 * - "ui": A2UI surface operations (creating/modifying UI components)
 * - "both": Can operate on both (future capability)
 */
export type AgentMode = "board" | "ui" | "both";

/**
 * Loading phases that represent the AI's current activity
 */
export type LoadingPhase =
	| "idle"
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

export const LOADING_PHASES: Record<LoadingPhase, LoadingPhaseInfo> = {
	idle: {
		label: "Ready",
		icon: null,
		color: "text-muted-foreground",
	},
	initializing: {
		label: "Starting up",
		icon: null,
		color: "text-blue-500",
	},
	analyzing: {
		label: "Analyzing...",
		icon: null,
		color: "text-violet-500",
	},
	searching: {
		label: "Searching...",
		icon: null,
		color: "text-cyan-500",
	},
	reasoning: {
		label: "Thinking...",
		icon: null,
		color: "text-amber-500",
	},
	generating: {
		label: "Generating...",
		icon: null,
		color: "text-pink-500",
	},
	finalizing: {
		label: "Finalizing...",
		icon: null,
		color: "text-green-500",
	},
};

/**
 * Image attachment interface used across both modes
 */
export interface AttachedImage {
	/** Base64-encoded image data (without data URL prefix) */
	data: string;
	/** MIME type (e.g., "image/png", "image/jpeg") */
	mediaType: string;
	/** Data URL for preview display */
	preview: string;
}

/**
 * Unified plan step that works for both board and UI modes
 */
export type UnifiedPlanStep = PlanStep | A2UIPlanStep;

/**
 * Unified message format for the copilot chat
 */
export interface CopilotMessage {
	role: "user" | "assistant";
	content: string;
	images?: AttachedImage[];
	/** Plan steps associated with this message */
	planSteps?: UnifiedPlanStep[];
	/** Context node IDs (board mode) */
	contextNodeIds?: string[];
	/** Applied components (UI mode) */
	appliedComponents?: SurfaceComponent[];
	/** Executed board commands (board mode) */
	executedCommands?: BoardCommand[];
}

/**
 * Props for the unified FlowPilot component
 */
export interface FlowPilotProps {
	/** The agent mode determines what the copilot operates on */
	agentMode: AgentMode;

	/** Title to display in the header (defaults to "FlowPilot") */
	title?: string;

	/** Custom class name for styling */
	className?: string;

	/** Callback when close button is clicked */
	onClose?: () => void;

	// === Board Mode Props ===

	/** The board to operate on (required for board mode) */
	board?: IBoard | null;

	/** Selected node IDs for context (board mode) */
	selectedNodeIds?: string[];

	/** Callback when a suggestion is accepted (board mode) */
	onAcceptSuggestion?: (suggestion: Suggestion) => void;

	/** Callback when commands should be executed (board mode) */
	onExecuteCommands?: (commands: BoardCommand[]) => void;

	/** Callback to focus on a specific node (board mode) */
	onFocusNode?: (nodeId: string) => void;

	/** Callback to select nodes (board mode) */
	onSelectNodes?: (nodeIds: string[]) => void;

	/** Run context for log analysis (board mode) */
	runContext?: {
		run_id: string;
		app_id: string;
		board_id: string;
		event_id?: string;
	};

	/** Initial prompt to auto-submit (board mode) */
	initialPrompt?: string;

	// === UI Mode Props ===

	/** Current UI components on the surface (UI mode) */
	currentComponents?: SurfaceComponent[];

	/** Selected component IDs (UI mode) */
	selectedComponentIds?: string[];

	/** Callback when components are generated (UI mode) */
	onComponentsGenerated?: (components: SurfaceComponent[]) => void;

	/** Callback when components should be applied (UI mode) */
	onApplyComponents?: (components: SurfaceComponent[]) => void;

	// === Screenshot Props ===

	/** Custom function to capture a screenshot. If provided, shows split send button.
	 * Should return base64 data URL of the screenshot, or null if capture failed. */
	captureScreenshot?: () => Promise<string | null>;
}

/**
 * Internal state interface for the FlowPilot component
 */
export interface FlowPilotState {
	messages: CopilotMessage[];
	input: string;
	loading: boolean;
	loadingPhase: LoadingPhase;
	loadingStartTime: number | null;
	elapsedSeconds: number;
	tokenCount: number;
	planSteps: UnifiedPlanStep[];
	attachedImages: AttachedImage[];
	userScrolledUp: boolean;
	selectedModelId: string;

	// Board-specific state
	pendingCommands: BoardCommand[];
	suggestions: Suggestion[];
	currentToolCall: string | null;

	// UI-specific state
	pendingComponents: SurfaceComponent[];
}
