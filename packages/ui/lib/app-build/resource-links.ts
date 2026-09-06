import { collectRunnableWorkflowEventEntries } from "../../components/global-chat/workflow-event-entries";
import type { IBackendState } from "../../state/backend-state";
import { EVENT_DEFINITIONS } from "../event-definitions";
import type { CompiledAppSpec } from "./compiler";

export function resourceId(plan: CompiledAppSpec, key: string): string {
	const resource = plan.resources.find((item) => item.key === key);
	if (!resource) throw new Error(`Unknown app resource '${key}'.`);
	return resource.physical_id;
}

/** Resolve a symbolic entry against persisted nodes, never a model-supplied success claim. */
export async function resolveBuildEntry(
	backend: IBackendState,
	appId: string,
	boardId: string,
	selector: string | undefined,
	eventType?: string,
): Promise<string> {
	const board = await backend.boardState.getBoardAuthoritative(
		appId,
		boardId,
		undefined,
	);
	const entries = collectRunnableWorkflowEventEntries(
		board,
		boardId,
		new Set(),
		(type) => EVENT_DEFINITIONS[type]?.eventTypes ?? [],
	);
	const matches = entries.filter(
		(entry) =>
			(!selector ||
				entry.id === selector ||
				entry.name === selector ||
				entry.node_type === selector) &&
			(!eventType || entry.supported_event_types.includes(eventType)),
	);
	if (matches.length !== 1)
		throw new Error(
			`Entry '${selector ?? "(unspecified)"}' resolves to ${matches.length} compatible runnable nodes. Name one exact entry in the app contract.`,
		);
	return matches[0].id;
}
