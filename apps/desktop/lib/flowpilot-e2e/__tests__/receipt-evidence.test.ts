import type {
	FlowScriptCompilerReceipt,
	FlowScriptGenerationRunReceipt,
} from "@flow-like/flow-like-ui/lib/flowpilot/flowscript-generation-receipt";
import { describe, expect, test } from "vitest";
import {
	authoredFlowScriptEvidence,
	findExactSuccessfulCompilerPair,
} from "../receipt-evidence";

const source = 'event submit() { log.info("ticket ✓"); }\n';

function inlineRun(): FlowScriptGenerationRunReceipt {
	const write: FlowScriptCompilerReceipt = {
		toolName: "write_flowscript",
		status: "draft_started",
		draftId: "ticket-draft",
		revision: 2,
		baseFingerprint: "base-board",
		source,
		diagnostics: [],
		reviewNotes: [],
		corrections: [],
		derivedCommandCount: 4,
		queuedCount: 0,
		success: false,
		payload: {
			status: "draft_started",
			draft_id: "ticket-draft",
			revision: 2,
			validation_sequence: 8,
			base_fingerprint: "base-board",
			source_fingerprint: "a".repeat(64),
			catalog_fingerprint: `b3:${"b".repeat(64)}`,
			commands_fingerprint: "c".repeat(64),
			source_bytes: new TextEncoder().encode(source).length,
			source_lines: 1,
			derived_command_count: 4,
			queued_count: 0,
		},
		capturedAtMs: 1,
	};
	const commit: FlowScriptCompilerReceipt = {
		...write,
		toolName: "commit_flowscript",
		status: "queued",
		queuedCount: 4,
		success: true,
		payload: {
			...write.payload,
			status: "queued",
			validation_sequence: 9,
			queued_count: 4,
		},
		capturedAtMs: 2,
	};
	return {
		schema: "flowpilot.flowscript-generation-run/v1",
		conversationId: "conversation",
		requestId: "request",
		appId: "app",
		boardId: "board",
		provider: "codex",
		modelId: "codex:gpt-5.6-terra",
		reasoningEffort: "high",
		startedAtMs: 0,
		endedAtMs: 3,
		outcome: "ok",
		candidates: [{ source, status: "queued", capturedAtMs: 2 }],
		compilerReceipts: [write, commit],
		appliedCommands: 4,
		persistedReadbackVerified: true,
	};
}

describe("exact compiler receipt evidence", () => {
	test("recognizes an inline checked commit without inventing an explicit check", () => {
		const run = inlineRun();
		expect(findExactSuccessfulCompilerPair(run)).toMatchObject({
			validationMode: "inline_commit",
			check: { toolName: "write_flowscript", success: false },
			commit: { toolName: "commit_flowscript", success: true },
		});
		expect(authoredFlowScriptEvidence([run])).toEqual({
			source,
			status: "queued",
			completion: undefined,
		});
	});

	test("accepts the exact retained patch source instead of projected byte counts", () => {
		const run = inlineRun();
		const retained = run.compilerReceipts[0];
		retained.toolName = "patch_flowscript";
		retained.status = "draft_updated";
		retained.payload = {
			...retained.payload,
			status: "draft_updated",
			source,
			source_bytes: undefined,
			source_lines: undefined,
		};
		expect(findExactSuccessfulCompilerPair(run)?.validationMode).toBe(
			"inline_commit",
		);
	});

	test.each([
		["draft_id", "another-draft"],
		["revision", 3],
		["base_fingerprint", "another-board"],
		["source_fingerprint", "d".repeat(64)],
		["catalog_fingerprint", `b3:${"d".repeat(64)}`],
		["commands_fingerprint", "d".repeat(64)],
		["commands_fingerprint", undefined],
		["source_bytes", source.length],
		["source_lines", 2],
		["validation_sequence", 8],
		["derived_command_count", 3],
		["queued_count", 3],
		["diagnostics", ["invalid call"]],
	])("rejects inconsistent commit envelope %s=%s", (key, value) => {
		const run = inlineRun();
		const commit = run.compilerReceipts[1];
		commit.payload = { ...commit.payload, [String(key)]: value };
		expect(findExactSuccessfulCompilerPair(run)).toBeNull();
		expect(authoredFlowScriptEvidence([run])).toEqual({});
	});

	test.each([
		"missing retained receipt",
		"mismatched source",
		"missing raw candidate",
		"unapplied commands",
		"unverified readback",
		"failed generation",
		"failed commit",
		"retained validation failure",
	])("rejects %s", (failure) => {
		const run = inlineRun();
		if (failure === "missing retained receipt")
			run.compilerReceipts = run.compilerReceipts.slice(1);
		if (failure === "mismatched source")
			run.compilerReceipts[1].source = source.replace("ticket", "thread");
		if (failure === "missing raw candidate") run.candidates = [];
		if (failure === "unapplied commands") run.appliedCommands = 3;
		if (failure === "unverified readback")
			run.persistedReadbackVerified = false;
		if (failure === "failed generation") run.outcome = "error";
		if (failure === "failed commit") run.compilerReceipts[1].success = false;
		if (failure === "retained validation failure")
			run.compilerReceipts[0].diagnostics = ["invalid call"];
		expect(findExactSuccessfulCompilerPair(run)).toBeNull();
	});
});
