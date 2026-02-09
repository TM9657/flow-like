// Use the new unified FlowPilot-based implementation
export { FlowCopilotWrapper as FlowCopilot } from "./FlowCopilotWrapper";
export type { FlowCopilotProps, LoadingPhase } from "./types";
export { LOADING_PHASES } from "./types";

// Legacy export - keeping for reference during migration
// To use old implementation, change: import { FlowCopilotWrapper as FlowCopilot } from "./FlowCopilotWrapper"
// to: import { FlowCopilot } from "./flow-copilot"
// export { FlowCopilot as FlowCopilotLegacy } from "./flow-copilot";

export type {
	AgentType,
	BoardCommand,
	ChatMessage,
	ChatRole,
	CopilotResponse,
	PlanStep,
	Suggestion,
} from "../../../lib/schema/flow/copilot";
