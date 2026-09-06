import type { AIProvider } from "../../components/flowpilot/types";
import {
	flowPilotModelIdForProvider,
	normalizeAIProvider,
} from "../../components/flowpilot/types";
import type { IBoardState } from "../../state/backend-state/board-state";
import type {
	OntologyQueryTarget,
	OntologyQueryTextCompletion,
} from "./ontology-query";

export interface FlowPilotOntologyQueryTextCompletionOptions {
	boardState: Pick<IBoardState, "copilot_chat" | "cancelCopilotChat">;
	target: OntologyQueryTarget;
	/** Provider selected by the host FlowPilot surface. */
	provider: AIProvider;
	/** Raw provider model id. The adapter adds the backend prefix when needed. */
	modelId?: string;
	reasoningEffort?: string;
	token?: string;
}

function abortError(): Error {
	const error = new Error("The ontology query generation was cancelled.");
	error.name = "AbortError";
	return error;
}

/**
 * Connects the ontology query controller to the existing FlowPilot transport.
 * DataStudio plus readOnly selects the tool-free ontology planner on both web
 * and desktop. The no-op token callback intentionally selects the web SSE path.
 */
export function createFlowPilotOntologyQueryTextCompletion(
	options: FlowPilotOntologyQueryTextCompletionOptions,
): OntologyQueryTextCompletion {
	const target = Object.freeze({ ...options.target });
	if (!target.appId.trim() || !target.overlayId.trim()) {
		throw new Error("Ontology query appId and overlayId are required.");
	}
	if (
		normalizeAIProvider(options.provider) !== "bits" &&
		!options.modelId?.trim()
	) {
		throw new Error(
			"A modelId is required for the selected FlowPilot agent backend.",
		);
	}
	const effectiveModelId = flowPilotModelIdForProvider(
		options.provider,
		options.modelId,
	);

	return async ({ requestId, sourcePrompt, userPrompt, signal }) => {
		if (!requestId.trim()) {
			throw new Error("Ontology query requestId is required.");
		}
		if (signal.aborted) throw abortError();

		let settled = false;
		const cancel = () => {
			if (settled) return;
			try {
				const cancellation = options.boardState.cancelCopilotChat?.(requestId);
				if (cancellation) void cancellation.catch(() => undefined);
			} catch {
				// Cancellation is best effort. The controller still suppresses the result.
			}
		};
		signal.addEventListener("abort", cancel, { once: true });

		try {
			const toolContext = {
				appId: target.appId,
				overlayId: target.overlayId,
				userScoped: target.userScoped,
				parentRequestId: requestId,
				conversationId: target.surfaceInstanceId,
				sourceUserPrompt: sourcePrompt,
			};
			const response = await options.boardState.copilot_chat(
				"DataStudio",
				null,
				undefined,
				[],
				null,
				null,
				[],
				userPrompt,
				[],
				undefined,
				() => undefined,
				effectiveModelId,
				options.reasoningEffort,
				options.token,
				undefined,
				undefined,
				true,
				true,
				toolContext,
				requestId,
				sourcePrompt,
				target.appId,
			);
			if (signal.aborted) throw abortError();
			return response.message;
		} finally {
			settled = true;
			signal.removeEventListener("abort", cancel);
		}
	};
}
