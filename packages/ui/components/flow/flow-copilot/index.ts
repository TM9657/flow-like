export { FlowCopilot } from "./flow-copilot";
export type { FlowCopilotProps, LoadingPhase } from "./types";
export { LOADING_PHASES } from "./types";

// Re-export types from lib/schema for backward compatibility
export type {
	AgentType,
	BoardCommand,
	ChatMessage,
	ChatRole,
	CopilotResponse,
	PlanStep,
	Suggestion,
} from "../../../lib/schema/flow/copilot";
