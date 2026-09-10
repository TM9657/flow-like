import type { AppScenarioRunResult } from "@flow-like/flow-like-ui/lib/app-build/scenarios";
import type { FlowScriptGenerationRunReceipt } from "@flow-like/flow-like-ui/lib/flowpilot/flowscript-generation-receipt";
import type {
	RetrievalComparisonEvidence,
	RetrievalPairMetrics,
} from "./retrieval-comparison";

export type FlowPilotE2ECaseId =
	| "simple-agent"
	| "forum"
	| "ops-dashboard"
	| "expense-approval"
	| "rss-digest"
	| "incident-console"
	| "mail-approval"
	| "doc-compliance"
	| "webhook-enrichment"
	| "agent-tools"
	| "multi-board-pages"
	| "ai-adventure"
	| "intake-reliability"
	| "retrieval-intake";

export type FlowPilotE2EReasoningEffort = "low" | "medium" | "high";
export type FlowPilotE2ETier = "structural" | "behavioral";

export interface FlowPilotE2EEvaluationIdentity {
	readonly harnessContractVersion: string;
	readonly caseSuiteFingerprint: string;
	readonly cohortKey: string;
	readonly provider: string;
	readonly model: string;
	readonly reasoningEffort: string;
	readonly tier: FlowPilotE2ETier;
}

/** Stable benchmark alias for a pinned generation model. */
export type FlowPilotE2EModelKey = "terra" | "sol";

export interface FlowPilotE2EModelConfig {
	provider: "codex";
	model: string;
	reasoningEffort: FlowPilotE2EReasoningEffort;
}

export type FlowPilotE2EEntityKind =
	| "page"
	| "widget"
	| "widget_action"
	| "table"
	| "event";

export type FlowScriptReferenceSource =
	| "authored"
	| "canonical"
	| "either"
	| "both";

/**
 * Identifies an entity by a stable semantic alias (normally its requested name).
 * The validator resolves the generated id at runtime and checks the selected
 * FlowScript source for that exact id.
 */
export interface FlowPilotE2ERequiredIdReference {
	entity: FlowPilotE2EEntityKind;
	alias: string;
	source?: FlowScriptReferenceSource;
}

export interface FlowPilotE2ERequiredNodeCapability {
	/** Stable report label for the behavior under test. */
	alias: string;
	/** Passing implementations may use any one of these persisted catalog node types. */
	anyOf: readonly string[];
}

export interface FlowPilotE2ERequiredPageBoardBinding {
	/** Semantic alias, name, or id of the persisted page. */
	page: string;
	/** Semantic alias, name, or id of the page's exact owning board. */
	board: string;
	/** Also require the page's onLoad node to exist on that same board. */
	requireOnLoadEvent?: boolean;
}

export interface FlowPilotE2ECaseRequirements {
	minFlowScriptNonWhitespaceChars: number;
	/** Guards the compact-output contract. The minimum is only a truncation check, not a target. */
	maxFlowScriptNonWhitespaceChars: number;
	minBoards: number;
	minTotalNodes: number;
	minPages: number;
	minWidgets: number;
	minTables: number;
	minEvents: number;
	requireAuthoredFlowScript: boolean;
	requireAuthoredLintDiagnostics: boolean;
	requireCanonicalFlowScript: boolean;
	requireLintDiagnostics: boolean;
	requireAuthoritativeReconcile: boolean;
	requireSuccessfulCompilerReceipt: boolean;
	validateReferenceIntegrity: boolean;
	requiredSemanticTableAliases: readonly string[];
	/**
	 * Database aliases that must be opened by canonical FlowScript rather than resolved from the
	 * setup-time table inventory. This preserves lazy stores whose schema depends on the first
	 * runtime value, such as an embedding vector's model-specific width.
	 */
	requiredLazyDatabaseAliases: readonly string[];
	requiredIdReferences: readonly FlowPilotE2ERequiredIdReference[];
	requiredNodeCapabilities: readonly FlowPilotE2ERequiredNodeCapability[];
	requiredPageBoardBindings: readonly FlowPilotE2ERequiredPageBoardBinding[];
}

export interface FlowPilotE2ECaseDefinition {
	id: FlowPilotE2ECaseId;
	title: string;
	description: string;
	appName: string;
	prompt: string;
	smoke: boolean;
	/**
	 * Wall clock the runner allows one turn of this case before abandoning it. Omission keeps the
	 * default; only raise it for cases whose scope genuinely needs more than one board build.
	 */
	runTimeoutMs?: number;
	requirements: FlowPilotE2ECaseRequirements;
}

/** A case with the unique app name and optional threshold overrides resolved. */
export interface ResolvedFlowPilotE2ECase extends FlowPilotE2ECaseDefinition {
	expectedAppName: string;
}

export interface BuildFlowPilotE2EPromptOptions {
	minFlowScriptNonWhitespaceChars?: number;
}

export interface BuiltFlowPilotE2EPrompt {
	caseDefinition: ResolvedFlowPilotE2ECase;
	prompt: string;
	expectedAppName: string;
}

export type FlowPilotLintSeverity =
	| "error"
	| "warning"
	| "info"
	| "hint"
	| (string & {});

export interface FlowPilotLintDiagnosticSnapshot {
	severity: FlowPilotLintSeverity;
	message: string;
	code?: string;
	line?: number;
	column?: number;
}

export interface FlowPilotReconcileSnapshot {
	parseValid: boolean;
	reconcileValid: boolean;
	idempotent?: boolean;
	commandCount?: number;
	corrections?: readonly string[];
	diagnostics?: readonly string[];
}

export interface FlowPilotBoardSnapshot {
	id: string;
	name: string;
	semanticAlias?: string;
	nodeCount?: number;
	/** Persisted node ids, used to verify page/app Event bindings. */
	nodeIds?: readonly string[];
	/** Persisted catalog node type names across the board root and every layer. */
	nodeTypes?: readonly string[];
	/** Canonical FlowScript read back from the persisted board. */
	flowScript?: string;
	/** Optional board-local authored source when a runner captures it per board. */
	authoredFlowScript?: string;
	/** Lint evidence for this board's exact authored source, before canonical readback. */
	authoredLintDiagnostics?: readonly FlowPilotLintDiagnosticSnapshot[];
	lintDiagnostics?: readonly FlowPilotLintDiagnosticSnapshot[];
	/** Optional authoritative compiler/reconciler result captured by the runner. */
	reconcile?: FlowPilotReconcileSnapshot;
}

export interface FlowPilotPageSnapshot {
	id: string;
	name: string;
	semanticAlias?: string;
	route?: string;
	boardId?: string;
	/** The persisted Event node id used by page-load execution. */
	onLoadEventId?: string;
	onUnloadEventId?: string;
	onIntervalEventId?: string;
	/** Kept intentionally opaque so browser, API and native collectors can share this type. */
	content?: unknown;
	/** Persisted A2UI definitions, retained for runtime failure diagnosis. */
	components?: unknown;
	/** Values may be ids, compact refs, or persisted widget definitions. */
	widgetRefs?: readonly unknown[] | Readonly<Record<string, unknown>>;
}

export interface FlowPilotWidgetActionSnapshot {
	id: string;
	name?: string;
	label?: string;
	semanticAlias?: string;
}

export interface FlowPilotWidgetSnapshot {
	id: string;
	name: string;
	semanticAlias?: string;
	actions?: readonly (string | FlowPilotWidgetActionSnapshot)[];
}

export interface FlowPilotTableSnapshot {
	id: string;
	name: string;
	semanticAlias?: string;
}

export interface FlowPilotEventSnapshot {
	id?: string;
	name?: string;
	semanticAlias?: string;
	boardId?: string;
	nodeId?: string;
	eventType?: string;
	pageId?: string;
}

/**
 * Portable artifact view produced by an E2E runner after app creation. Strings
 * in `tables` are accepted for collectors that only expose table names.
 */
export interface FlowPilotAppCreationSnapshot {
	appId: string;
	appName: string;
	/** Actual generation configuration. Omission is retained in the type so malformed runs report
	 * a validation failure instead of throwing before an artifact can be saved. */
	model?: FlowPilotE2EModelConfig;
	/** Exact model-authored source retained from the generation trace. */
	authoredFlowScript?: string;
	authoredFlowScriptStatus?: string;
	authoredFlowScriptCompletion?: string;
	authoredLintDiagnostics?: readonly FlowPilotLintDiagnosticSnapshot[];
	/** Turn-local raw candidates and exact compiler envelopes, scoped to their app and board. */
	flowScriptGenerationRuns?: readonly FlowScriptGenerationRunReceipt[];
	boards: readonly FlowPilotBoardSnapshot[];
	pages: readonly FlowPilotPageSnapshot[];
	widgets: readonly FlowPilotWidgetSnapshot[];
	tables: readonly (string | FlowPilotTableSnapshot)[];
	events: readonly FlowPilotEventSnapshot[];
	/** Host-produced scenario results. Omission is expected for the default structural tier. */
	behavioralScenarioResults?: readonly AppScenarioRunResult[];
}

export type FlowPilotE2ECheckStatus = "pass" | "fail";

export interface FlowPilotE2ECheck {
	code: string;
	status: FlowPilotE2ECheckStatus;
	message: string;
	path?: string;
	expected?: string | number | boolean;
	actual?: string | number | boolean;
}

export interface FlowScriptSizeMetrics {
	characters: number;
	nonWhitespaceCharacters: number;
	lines: number;
	/** A deterministic rough size signal, not a tokenizer-specific token count. */
	estimatedTokens: number;
}

export interface FlowPilotBoardFlowScriptMetrics extends FlowScriptSizeMetrics {
	boardId: string;
	boardName: string;
}

export interface FlowPilotE2EBehavioralMetrics {
	scenarios: number;
	passedScenarios: number;
	failedScenarios: number;
	blockedScenarios: number;
	unknownScenarios: number;
	startedRuns: number;
	successfulRuns: number;
	failedRuns: number;
	unknownRuns: number;
}

export interface FlowPilotE2ERunReport {
	schema: "flowpilot.app-creation-e2e-report/v1";
	caseId: FlowPilotE2ECaseId;
	caseTitle: string;
	appId: string;
	appName: string;
	expectedAppName: string;
	model: FlowPilotE2EModelConfig;
	evaluationIdentity: FlowPilotE2EEvaluationIdentity;
	/** Omitted by legacy artifacts; omission means structural. */
	tier?: FlowPilotE2ETier;
	passed: boolean;
	summary: {
		checks: number;
		passed: number;
		failed: number;
	};
	inventory: {
		boards: number;
		totalNodes: number;
		pages: number;
		widgets: number;
		tables: number;
		events: number;
	};
	flowScript: {
		authored?: FlowScriptSizeMetrics;
		canonical: readonly FlowPilotBoardFlowScriptMetrics[];
	};
	behavioral?: FlowPilotE2EBehavioralMetrics;
	checks: readonly FlowPilotE2ECheck[];
	failures: readonly FlowPilotE2ECheck[];
}

export interface FlowPilotE2ERunOptions {
	runtimeSourceFingerprint?: string;
	retrievalComparison?: boolean;
	retrievalRuntimeFingerprint?: string;
	/** Backwards-compatible single-case selector used by the browser console API. */
	caseId?: FlowPilotE2ECaseId;
	/** Ordered case selection used by the CLI. */
	caseIds?: readonly FlowPilotE2ECaseId[];
	suite?: "smoke" | "full";
	/** Benchmark model alias; omission keeps the default pinned model. */
	modelKey?: FlowPilotE2EModelKey;
	/** Omission preserves the existing compile/structure-only benchmark. */
	tier?: FlowPilotE2ETier;
	minFlowScriptNonWhitespaceChars?: number;
	repeat?: number;
	/**
	 * Cases to keep in flight at once. The chat caps concurrent runs, so this is bounded by the
	 * runner. Parallel runs and `failFast` are mutually exclusive: a fail-fast stop is only
	 * meaningful when the next case has not started yet.
	 */
	concurrency?: number;
	failFast?: boolean;
}

export interface FlowPilotE2ERunnerIssue {
	code: string;
	message: string;
}

export interface FlowPilotE2EAssistantTrace {
	id?: string;
	content?: unknown;
	appRefs?: string[];
	planSteps?: unknown;
	usageStats?: unknown;
	debugReport?: unknown;
}

export interface FlowPilotE2EArtifact {
	reliability?: {
		scope: "host_provisioned_intake";
		runtimeSourceFingerprint?: string;
		foundation?: unknown;
		eventRegistration?: unknown;
		runtime?: unknown;
		stages: {
			phase:
				| "provisioning"
				| "generation"
				| "registration"
				| "collection"
				| "behavioral";
			startedAtMs: number;
			endedAtMs: number;
			status: "ok" | "error";
			error?: string;
		}[];
	};
	retrievalComparison?: RetrievalComparisonEvidence;
	schema: "flowpilot.app-creation-e2e-artifact/v1";
	generatedAt: string;
	durationMs: number;
	requestedModelKey: FlowPilotE2EModelKey;
	requestedModel: FlowPilotE2EModelConfig;
	/** Omitted by legacy artifacts; omission means structural. */
	requestedTier?: FlowPilotE2ETier;
	observedModel?: {
		provider: string;
		model: string;
		reasoningEffort: string;
	};
	caseId: FlowPilotE2ECaseId;
	expectedAppName: string;
	prompt: string;
	snapshot?: FlowPilotAppCreationSnapshot;
	flowScriptGenerationRuns?: readonly FlowScriptGenerationRunReceipt[];
	assistantTrace?: FlowPilotE2EAssistantTrace;
	runner: {
		suppressedNavigations: string[];
		issues: FlowPilotE2ERunnerIssue[];
	};
	report?: FlowPilotE2ERunReport;
	/** Stable grouping key for repeated tight-loop failures across generated ids. */
	failureFingerprint?: string;
	error?: string;
}

export interface FlowPilotE2ECliEnvelope {
	reliabilitySummary?: ReturnType<
		typeof import("./reliability-metrics").summarizeIntakeReliability
	>;
	retrievalComparisonResults?: RetrievalPairMetrics[];
	schema: "flowpilot.app-creation-e2e-cli-result/v1";
	runId: string;
	startedAt: string;
	completedAt: string;
	durationMs: number;
	selection: {
		runtimeSourceFingerprint?: string;
		retrievalComparison?: boolean;
		retrievalRuntimeFingerprint?: string;
		caseIds: readonly FlowPilotE2ECaseId[];
		modelKey: FlowPilotE2EModelKey;
		/** Omitted by legacy callbacks; omission means structural. */
		tier?: FlowPilotE2ETier;
		evaluationIdentity?: FlowPilotE2EEvaluationIdentity;
		repeat: number;
		minFlowScriptNonWhitespaceChars?: number;
		failFast: boolean;
	};
	artifacts: readonly FlowPilotE2EArtifact[];
	passed: boolean;
	summary: {
		requestedRuns: number;
		completedRuns: number;
		passed: number;
		failed: number;
		skipped: number;
		behavioral?: FlowPilotE2EBehavioralMetrics;
	};
	/** Infrastructure or runner failure outside a completed case report. */
	error?: string;
}
