import type {
	WorkspaceEvaluationCall,
	WorkspaceEvaluationMode,
} from "@flow-like/flow-like-ui/lib/flowpilot/workspace-evaluation";
import { workspaceRevision } from "@flow-like/flow-like-ui/lib/flowpilot/workspace-resource";
import type { FlowPilotE2EArtifact } from "./types";

export interface RetrievalComparisonContext {
	pairId: string;
	round: number;
	mode: WorkspaceEvaluationMode;
	order: number;
	runtimeFingerprint: string;
	seed: RetrievalSeed;
}
export interface RetrievalSeed {
	appId: string;
	fixtureHash: string;
	canonicalHash: string;
	boards: { id: string; name: string; revision: string }[];
}
export interface RetrievalComparisonEvidence
	extends RetrievalComparisonContext {
	canonicalHashAfter?: string;
	sourceUnchanged: boolean;
	calls: WorkspaceEvaluationCall[];
}
export function retrievalPairOrder(round: number): WorkspaceEvaluationMode[] {
	if (!Number.isSafeInteger(round) || round < 0)
		throw new Error("Invalid comparison round.");
	return round % 2 === 0 ? ["baseline", "improved"] : ["improved", "baseline"];
}
export async function retrievalSeedHash(
	sources: readonly { name: string; source: string }[],
) {
	return workspaceRevision(JSON.stringify(sources));
}

function numeric(value: unknown): number | null {
	return typeof value === "number" && Number.isFinite(value) && value >= 0
		? value
		: null;
}
function record(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}
export function retrievalArtifactMetrics(artifact: FlowPilotE2EArtifact) {
	const calls = artifact.retrievalComparison?.calls ?? [];
	const usageEntries = Array.isArray(artifact.assistantTrace?.usageStats)
		? artifact.assistantTrace.usageStats
		: [];
	const tokenSum = (field: string) => {
		const values = usageEntries.map((entry) =>
			numeric(record(record(record(entry).stats).usage)[field]),
		);
		return values.length && values.every((value) => value !== null)
			? values.reduce<number>((sum, value) => sum + (value ?? 0), 0)
			: null;
	};
	const generationRuns = artifact.flowScriptGenerationRuns ?? [];
	const checks = artifact.report?.checks ?? [];
	const wiring = checks.filter((check) =>
		/reference|binding|capability|event|page/i.test(check.code),
	);
	return {
		mode: artifact.retrievalComparison?.mode,
		pairId: artifact.retrievalComparison?.pairId,
		elapsedMs: artifact.durationMs,
		structuralCompleted: Boolean(artifact.report?.passed) && !artifact.error,
		completed: Boolean(artifact.report?.passed) && !artifact.error,
		passedRequirements: artifact.report?.summary.passed ?? null,
		failedRequirements: artifact.report?.summary.failed ?? null,
		failedWiringChecks: wiring.filter((check) => check.status === "fail")
			.length,
		observedWiringChecks: wiring.length,
		generationAttempts: generationRuns.length,
		// Candidate/receipt counts are observable; they are not a count of agent repair decisions.
		repairDecisions: null,
		inputTokens: tokenSum("prompt_tokens"),
		outputTokens: tokenSum("completion_tokens"),
		retrievalCalls: calls.length,
		retrievalDurationMs: calls.reduce(
			(total, call) => total + call.durationMs,
			0,
		),
		retrievalResponseBytes: calls.reduce(
			(total, call) => total + call.responseBytes,
			0,
		),
		retrievalExercised: calls.some((call) => call.tool === "search_workspace"),
		sourceUnchanged: artifact.retrievalComparison?.sourceUnchanged ?? null,
		behavioralAcceptance: artifact.report?.behavioral ?? null,
	};
}

export interface RetrievalPairMetrics {
	pairId: string;
	baseline: ReturnType<typeof retrievalArtifactMetrics>;
	improved: ReturnType<typeof retrievalArtifactMetrics>;
}

export function compareRetrievalArtifacts(
	artifacts: readonly FlowPilotE2EArtifact[],
): RetrievalPairMetrics[] {
	const pairs = new Map<string, FlowPilotE2EArtifact[]>();
	for (const artifact of artifacts) {
		const evidence = artifact.retrievalComparison;
		if (!evidence) throw new Error("Retrieval comparison evidence is missing.");
		pairs.set(evidence.pairId, [
			...(pairs.get(evidence.pairId) ?? []),
			artifact,
		]);
	}
	return [...pairs.entries()].map(([pairId, pair]) => {
		const baseline = pair.find(
			(item) => item.retrievalComparison?.mode === "baseline",
		);
		const improved = pair.find(
			(item) => item.retrievalComparison?.mode === "improved",
		);
		if (pair.length !== 2 || !baseline || !improved)
			throw new Error(`Incomplete retrieval pair ${pairId}.`);
		const a = baseline.retrievalComparison;
		const b = improved.retrievalComparison;
		if (!a || !b) throw new Error(`Missing retrieval evidence in ${pairId}.`);
		if (
			a.seed.appId !== b.seed.appId ||
			a.seed.fixtureHash !== b.seed.fixtureHash ||
			a.seed.canonicalHash !== b.seed.canonicalHash ||
			a.runtimeFingerprint !== b.runtimeFingerprint ||
			JSON.stringify(baseline.requestedModel) !==
				JSON.stringify(improved.requestedModel) ||
			baseline.caseId !== improved.caseId ||
			baseline.prompt.split(baseline.expectedAppName).join("<destination>") !==
				improved.prompt.split(improved.expectedAppName).join("<destination>") ||
			JSON.stringify(baseline.observedModel) !==
				JSON.stringify(improved.observedModel) ||
			!a.sourceUnchanged ||
			!b.sourceUnchanged ||
			a.canonicalHashAfter !== a.seed.canonicalHash ||
			b.canonicalHashAfter !== b.seed.canonicalHash ||
			(baseline.snapshot &&
				improved.snapshot &&
				baseline.snapshot.appId === improved.snapshot.appId)
		) {
			throw new Error(
				`Retrieval pair ${pairId} changed a controlled dimension.`,
			);
		}
		return {
			pairId,
			baseline: retrievalArtifactMetrics(baseline),
			improved: retrievalArtifactMetrics(improved),
		};
	});
}
