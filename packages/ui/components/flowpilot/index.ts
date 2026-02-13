/**
 * FlowPilot - Unified AI Copilot for Board and UI modes
 * Export all FlowPilot-related components and utilities
 */

// Main unified component
export { FlowPilot } from "./FlowPilot";

// Types
export type {
	AgentMode,
	AttachedImage,
	CopilotMessage,
	FlowPilotProps,
	LoadingPhase,
	UnifiedPlanStep,
} from "./types";
export { LOADING_PHASES } from "./types";

// Sub-components (can be used standalone)
export { ContextNodes } from "./ContextNodes";
export { MessageContent } from "./MessageContent";
export { PendingCommandsView } from "./PendingCommandsView";
export { PendingComponentsView } from "./PendingComponentsView";
export { PlanStepsView } from "./PlanStepsView";
export { StatusPill, LOADING_PHASE_CONFIG } from "./StatusPill";

// Utilities
export {
	detectAgentMode,
	getCommandColor,
	getCommandIcon,
	getCommandSummary,
	getComponentCounts,
	getComponentIcon,
	getComponentSummary,
} from "./utils";

// Legacy exports (for backward compatibility)
export { A2UIPreview } from "./A2UIPreview";
export type { A2UIPreviewProps } from "./A2UIPreview";

export { FlowPilotInput } from "./FlowPilotInput";
export type { FlowPilotInputProps, FlowPilotMode } from "./FlowPilotInput";
