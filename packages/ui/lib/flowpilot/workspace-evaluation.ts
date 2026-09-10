import type { IBackendState } from "../../state/backend-state";
import type { WorkspaceScope } from "./workspace-content";
import { createWorkspaceIndex as createBaselineIndex } from "./workspace-ranker-baseline";
import { jsonBytes, parseWorkspaceResourceId } from "./workspace-resource";
import { WorkspaceSearchSession } from "./workspace-search";

export type WorkspaceEvaluationMode = "baseline" | "improved";
export interface WorkspaceEvaluationCall {
	tool: string;
	query?: string;
	appId?: string;
	startedAt: number;
	durationMs: number;
	responseBytes: number;
	status: unknown;
	matchMode?: unknown;
	hits?: number;
	freshness?: unknown;
}
export interface WorkspaceEvaluation {
	mode: WorkspaceEvaluationMode;
	/** Present only when a retrieval comparison has a read-only source fixture. */
	sourceAppId?: string;
	destinationAppIds: Set<string>;
	session: WorkspaceSearchSession;
	calls: WorkspaceEvaluationCall[];
}
const evaluations = new Map<string, WorkspaceEvaluation>();

function registerEvaluation(
	conversationId: string,
	mode: WorkspaceEvaluationMode,
	sourceAppId: string | undefined,
	destinationAppIds: string[] = [],
): WorkspaceEvaluation {
	if (process.env.NODE_ENV !== "development") {
		throw new Error("Workspace evaluations require a development runtime.");
	}
	if (evaluations.has(conversationId))
		throw new Error("Evaluation already registered.");
	const evaluation: WorkspaceEvaluation = {
		mode,
		...(sourceAppId === undefined ? {} : { sourceAppId }),
		destinationAppIds: new Set(destinationAppIds),
		session: new WorkspaceSearchSession(
			Date.now,
			mode === "baseline"
				? { createIndex: createBaselineIndex, cacheEnabled: false }
				: {},
		),
		calls: [],
	};
	evaluations.set(conversationId, evaluation);
	return evaluation;
}

/** Only the development runner can select an arm; model tool arguments never select it. */
export function registerWorkspaceEvaluation(
	conversationId: string,
	mode: WorkspaceEvaluationMode,
	sourceAppId: string,
): WorkspaceEvaluation {
	return registerEvaluation(conversationId, mode, sourceAppId);
}

/** The host admits an already provisioned app for a scoped build without a read-only source. */
export function registerWorkspaceBuildEvaluation(
	conversationId: string,
	appId: string,
): WorkspaceEvaluation {
	if (!appId.trim()) throw new Error("A build evaluation requires an app ID.");
	return registerEvaluation(conversationId, "improved", undefined, [appId]);
}

export function getWorkspaceEvaluation(conversationId?: string) {
	return conversationId ? evaluations.get(conversationId) : undefined;
}
export function unregisterWorkspaceEvaluation(conversationId: string) {
	evaluations.delete(conversationId);
}

export function registerWorkspaceEvaluationDestination(
	evaluation: WorkspaceEvaluation,
	appId: string,
) {
	if (!appId.trim() || appId === evaluation.sourceAppId)
		throw new Error("Invalid evaluation destination.");
	evaluation.destinationAppIds.add(appId);
}
export function workspaceEvaluationAllowsApp(
	evaluation: WorkspaceEvaluation,
	appId: string,
) {
	return (
		appId === evaluation.sourceAppId || evaluation.destinationAppIds.has(appId)
	);
}
function nonempty(value: unknown): string | undefined {
	return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

const SOURCE_READ_TOOLS = new Set([
	"create_app",
	"list_apps",
	"search_apps",
	"search_templates",
	"get_template_preview",
	"fork_preview",
	"search_nodes",
	"get_node_definitions",
	"get_type_details",
	"project_scout",
	"describe_app_interface",
	"search_workspace",
	"read_symbol",
	"inspect_app",
	"get_app_detail",
	"get_app_context",
	"get_board_context",
	"get_board_flow_script",
	"get_flow_script",
	"get_flow_script_file",
	"get_board",
	"get_board_details",
	"get_node_details",
	"list_boards",
	"list_events",
	"list_pages",
	"get_page",
	"list_tables",
	"get_table_schema",
	"get_event",
	"get_scoped_flow_script",
	"read_flow_script",
	"read_flowscript_source",
	"query_app_data",
]);
export function workspaceEvaluationMutationError(
	evaluation: WorkspaceEvaluation | undefined,
	toolName: string,
	args: Record<string, unknown>,
	contextAppId?: string,
): Record<string, unknown> | undefined {
	if (
		!evaluation?.sourceAppId ||
		SOURCE_READ_TOOLS.has(toolName) ||
		(toolName === "flowpilot_board" &&
			(args.mode === "explain" || args.mode === "inspect")) ||
		(toolName === "flowpilot_widget" && args.mode === "inspect") ||
		(toolName === "app_build" &&
			["schema", "capabilities", "recipe", "status"].includes(
				String(args.operation),
			))
	)
		return undefined;
	const targetAppId =
		nonempty(args.app_id) || nonempty(args.appId) || nonempty(contextAppId);
	if (targetAppId !== evaluation.sourceAppId) return undefined;
	return {
		status: "error",
		code: "EVALUATION_SOURCE_READ_ONLY",
		message:
			"This source app is a read-only evaluation fixture. Create and edit the requested destination app.",
	};
}

export function workspaceEvaluationTargetError(
	evaluation: WorkspaceEvaluation | undefined,
	toolName: string,
	args: Record<string, unknown>,
	resolvedAppId?: string,
): Record<string, unknown> | undefined {
	if (
		evaluation &&
		!evaluation.sourceAppId &&
		["create_app", "upsert_event"].includes(toolName)
	) {
		return {
			status: "error",
			code: "EVALUATION_HOST_PROVISIONED",
			message:
				"The host owns app creation and Event registration for this build. Complete the page and workflow in the supplied app.",
		};
	}
	if (!evaluation || toolName === "create_app") return undefined;
	const contextIndependent =
		[
			"list_apps",
			"search_apps",
			"search_templates",
			"get_template_preview",
			"search_nodes",
			"get_node_definitions",
			"get_type_details",
		].includes(toolName) ||
		(toolName === "app_build" &&
			["schema", "capabilities", "recipe"].includes(String(args.operation)));
	const target =
		(contextIndependent ? undefined : nonempty(resolvedAppId)) ||
		nonempty(args.app_id) ||
		nonempty(args.appId);
	if (target && !workspaceEvaluationAllowsApp(evaluation, target))
		return {
			status: "error",
			code: "EVALUATION_APP_OUT_OF_SCOPE",
			message: "This evaluation can access only its registered apps.",
		};
	return workspaceEvaluationMutationError(
		evaluation,
		toolName,
		target ? { ...args, app_id: target, appId: target } : args,
	);
}

export async function dispatchWorkspaceEvaluationRead(
	evaluation: WorkspaceEvaluation,
	tool: "search_workspace" | "read_symbol",
	backend: IBackendState,
	args: Record<string, unknown>,
	scope: WorkspaceScope,
): Promise<Record<string, unknown>> {
	const startedAt = Date.now();
	// Recheck membership while excluding apps left by other evaluation runs.
	const scoped: WorkspaceScope = {
		...scope,
		scopedAppId:
			scope.scopedAppId &&
			workspaceEvaluationAllowsApp(evaluation, scope.scopedAppId)
				? scope.scopedAppId
				: undefined,
		getProfileAppIds: async () =>
			new Set(
				[...(await scope.getProfileAppIds())].filter((id) =>
					workspaceEvaluationAllowsApp(evaluation, id),
				),
			),
	};
	const result =
		tool === "search_workspace"
			? await evaluation.session.search(backend, args, scoped)
			: await evaluation.session.readSymbol(backend, args, scoped);
	evaluation.calls.push({
		tool,
		query: typeof args.query === "string" ? args.query : undefined,
		appId:
			tool === "read_symbol"
				? typeof result.app_id === "string"
					? result.app_id
					: parseWorkspaceResourceId(args.resource_id)?.app_id
				: typeof args.app_id === "string"
					? args.app_id
					: scoped.scopedAppId,
		startedAt,
		durationMs: Date.now() - startedAt,
		responseBytes: jsonBytes(result),
		status: result.status,
		matchMode: result.match_mode,
		hits: Array.isArray(result.hits) ? result.hits.length : undefined,
		freshness: result.freshness,
	});
	return result;
}
