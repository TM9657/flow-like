import {
	type FlowScriptCompilerReceipt,
	type FlowScriptGenerationRunReceipt,
	isSuccessfulFlowScriptCheckReceipt,
	isSuccessfulFlowScriptCommitReceipt,
} from "@flow-like/flow-like-ui/lib/flowpilot/flowscript-generation-receipt";

export interface ExactSuccessfulCompilerPair {
	check: FlowScriptCompilerReceipt;
	commit: FlowScriptCompilerReceipt;
	validationMode: "separate_check" | "inline_commit";
}

function hasExactDraftEnvelope(receipt: FlowScriptCompilerReceipt): boolean {
	const payload = receipt.payload;
	return (
		Boolean(receipt.source?.trim()) &&
		Boolean(receipt.draftId) &&
		Number.isSafeInteger(receipt.revision) &&
		Number(receipt.revision) >= 0 &&
		payload.draft_id === receipt.draftId &&
		payload.revision === receipt.revision &&
		payload.status === receipt.status &&
		Boolean(receipt.baseFingerprint) &&
		payload.base_fingerprint === receipt.baseFingerprint &&
		Number.isSafeInteger(payload.validation_sequence) &&
		Number(payload.validation_sequence) > 0 &&
		/^[a-f0-9]{64}$/.test(String(payload.source_fingerprint)) &&
		/^b3:[a-f0-9]{64}$/.test(String(payload.catalog_fingerprint)) &&
		/^[a-f0-9]{64}$/.test(String(payload.commands_fingerprint)) &&
		Number.isSafeInteger(receipt.derivedCommandCount) &&
		Number(receipt.derivedCommandCount) >= 0 &&
		payload.derived_command_count === receipt.derivedCommandCount &&
		payload.queued_count === receipt.queuedCount &&
		receipt.diagnostics.length === 0 &&
		(payload.diagnostics === undefined ||
			(Array.isArray(payload.diagnostics) &&
				payload.diagnostics.length === 0)) &&
		(payload.source === receipt.source ||
			(payload.source_bytes ===
				new TextEncoder().encode(receipt.source).length &&
				payload.source_lines ===
					receipt.source?.replace(/\n$/, "").split("\n").length))
	);
}

function matchingInlineValidation(
	checked: FlowScriptCompilerReceipt,
	commit: FlowScriptCompilerReceipt,
): boolean {
	return (
		((checked.toolName === "write_flowscript" &&
			checked.status === "draft_started") ||
			(checked.toolName === "patch_flowscript" &&
				checked.status === "draft_updated")) &&
		hasExactDraftEnvelope(checked) &&
		checked.source === commit.source &&
		checked.draftId === commit.draftId &&
		checked.revision === commit.revision &&
		checked.derivedCommandCount === commit.derivedCommandCount &&
		checked.queuedCount === 0 &&
		Number(checked.payload.validation_sequence) <
			Number(commit.payload.validation_sequence) &&
		[
			"base_fingerprint",
			"source_fingerprint",
			"catalog_fingerprint",
			"commands_fingerprint",
		].every((key) => checked.payload[key] === commit.payload[key])
	);
}

/**
 * Match an explicit check or a retained validation followed by a checked commit.
 * The native commit also checks inline. Its exact envelope must match the retained
 * source and batch identity; a generic queued result is insufficient evidence.
 */
export function findExactSuccessfulCompilerPair(
	run: FlowScriptGenerationRunReceipt,
): ExactSuccessfulCompilerPair | null {
	if (run.outcome !== "ok" || run.persistedReadbackVerified !== true) {
		return null;
	}

	for (let index = run.compilerReceipts.length - 1; index >= 0; index -= 1) {
		const commit = run.compilerReceipts[index];
		if (!commit || !isSuccessfulFlowScriptCommitReceipt(commit)) continue;
		const check = run.compilerReceipts
			.slice(0, index)
			.findLast(
				(candidate) =>
					isSuccessfulFlowScriptCheckReceipt(candidate) &&
					candidate.source === commit.source &&
					Boolean(candidate.draftId) &&
					candidate.draftId === commit.draftId &&
					candidate.revision !== undefined &&
					candidate.revision === commit.revision,
			);
		if (check) return { check, commit, validationMode: "separate_check" };
		if (
			commit.toolName !== "commit_flowscript" ||
			commit.status !== "queued" ||
			!hasExactDraftEnvelope(commit) ||
			commit.queuedCount !== commit.derivedCommandCount ||
			run.appliedCommands !== commit.queuedCount ||
			!run.candidates.some((candidate) => candidate.source === commit.source)
		)
			continue;
		const retained = run.compilerReceipts
			.slice(0, index)
			.findLast((candidate) => matchingInlineValidation(candidate, commit));
		if (retained)
			return { check: retained, commit, validationMode: "inline_commit" };
	}

	return null;
}

export function authoredFlowScriptEvidence(
	runs: readonly FlowScriptGenerationRunReceipt[],
): { source?: string; status?: string; completion?: string } {
	for (let index = runs.length - 1; index >= 0; index -= 1) {
		const run = runs[index];
		if (!run) continue;
		const pair = findExactSuccessfulCompilerPair(run);
		if (!pair?.commit.source) continue;
		const candidate = [...run.candidates]
			.reverse()
			.find((entry) => entry.source === pair.commit.source);
		return {
			source: pair.commit.source,
			status: pair.commit.status ?? candidate?.status,
			completion: candidate?.completion,
		};
	}

	return {};
}
