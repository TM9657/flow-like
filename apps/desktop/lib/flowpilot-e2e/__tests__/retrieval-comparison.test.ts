import type { IBackendState } from "@flow-like/flow-like-ui/state/backend-state";
import { describe, expect, it } from "vitest";
import { parseArgs } from "../../../scripts/flowpilot-e2e";
import {
	type RetrievalComparisonEvidence,
	compareRetrievalArtifacts,
	retrievalArtifactMetrics,
	retrievalPairOrder,
} from "../retrieval-comparison";
import { createRetrievalSeed, readRetrievalSeedHash } from "../retrieval-seed";
import type { FlowPilotE2EArtifact } from "../types";

function artifact(
	mode: "baseline" | "improved",
	patch: Partial<RetrievalComparisonEvidence> = {},
): FlowPilotE2EArtifact {
	return {
		schema: "flowpilot.app-creation-e2e-artifact/v1",
		generatedAt: "2026-09-09",
		durationMs: 500,
		caseId: "retrieval-intake",
		expectedAppName: `Target ${mode}`,
		prompt: "same task",
		requestedModelKey: "terra",
		requestedModel: {
			provider: "codex",
			model: "gpt-5.6-terra",
			reasoningEffort: "high",
		},
		runner: { issues: [], suppressedNavigations: [] },
		retrievalComparison: {
			pairId: "pair",
			round: 0,
			order: mode === "baseline" ? 0 : 1,
			mode,
			runtimeFingerprint: "runtime",
			seed: {
				appId: "source",
				fixtureHash: "fixture",
				canonicalHash: "canonical",
				boards: [],
			},
			canonicalHashAfter: "canonical",
			sourceUnchanged: true,
			calls: [],
			...patch,
		},
	};
}
describe("paired live retrieval contract", () => {
	it("alternates arm order across rounds and rejects invalid rounds", () => {
		expect(retrievalPairOrder(0)).toEqual(["baseline", "improved"]);
		expect(retrievalPairOrder(1)).toEqual(["improved", "baseline"]);
		expect(() => retrievalPairOrder(-1)).toThrow();
	});
	it("pins a serial seeded comparison from CLI", () => {
		expect(
			parseArgs(["--retrieval-ab", "--isolated", "--repeat", "2"]),
		).toMatchObject({
			caseIds: ["retrieval-intake"],
			repeat: 2,
			concurrency: 1,
			retrievalComparison: true,
			isolated: true,
		});
		for (const args of [
			["--concurrency", "2"],
			["--fail-fast"],
			["--tier", "behavioral"],
			["--case", "forum"],
		]) {
			expect(() => parseArgs(["--retrieval-ab", ...args])).toThrow();
		}
	});
	it("compares paired failures without pretending absent observations are zero", () => {
		const compared = compareRetrievalArtifacts([
			artifact("baseline"),
			artifact("improved"),
		]);
		expect(compared[0].baseline).toMatchObject({
			completed: false,
			inputTokens: null,
			outputTokens: null,
			repairDecisions: null,
			retrievalExercised: false,
			behavioralAcceptance: null,
		});
	});
	it("sums observed token receipts while retaining absent token evidence", () => {
		const observed = artifact("baseline");
		observed.assistantTrace = {
			usageStats: [
				{ stats: { usage: { prompt_tokens: 10, completion_tokens: 2 } } },
				{ stats: { usage: { prompt_tokens: 20, completion_tokens: 4 } } },
			],
		};
		expect(retrievalArtifactMetrics(observed)).toMatchObject({
			inputTokens: 30,
			outputTokens: 6,
		});
	});
	it("rejects incomplete pairs and changed controlled dimensions", () => {
		expect(() => compareRetrievalArtifacts([artifact("baseline")])).toThrow(
			"Incomplete",
		);
		for (const patch of [
			{ runtimeFingerprint: "other" },
			{ sourceUnchanged: false },
			{ canonicalHashAfter: "changed" },
			{
				seed: {
					appId: "other",
					fixtureHash: "fixture",
					canonicalHash: "canonical",
					boards: [],
				},
			},
		]) {
			expect(() =>
				compareRetrievalArtifacts([
					artifact("baseline"),
					artifact("improved", patch),
				]),
			).toThrow("controlled dimension");
		}
	});
	it("rejects failed applies and empty or unrelated canonical output before generation", async () => {
		for (const fixture of [
			{
				commands: [],
				diagnostics: ["compile rejected"],
				canonical: "",
				expected: "did not apply successfully",
			},
			{
				commands: [],
				diagnostics: [],
				canonical: "",
				expected: "did not apply successfully",
			},
			{
				commands: [{}],
				diagnostics: [],
				canonical: "",
				expected: "lacks its expected",
			},
			{
				commands: [{}],
				diagnostics: [],
				canonical: "eventsSimple unrelated() {}",
				expected: "lacks its expected",
			},
		]) {
			const backend = {
				appState: { createApp: async () => ({ id: "source" }) },
				userState: {
					getSettingsProfile: async () => ({}),
					updateProfileApp: async () => {},
				},
				boardState: {
					getBoardSummariesAuthoritative: async () => [],
					upsertBoard: async () => {},
					getFlowScriptAuthoritative: async () => fixture.canonical,
					applyFlowScript: async () => ({
						commands: fixture.commands,
						board_commands: [],
						diagnostics: fixture.diagnostics,
					}),
					checkFlowScriptReconcile: async () => {
						throw new Error("Invalid source should fail before reconciliation");
					},
				},
			} as unknown as IBackendState;
			await expect(createRetrievalSeed(backend, "pair")).rejects.toThrow(
				fixture.expected,
			);
		}
	});
	it("creates only a local source and verifies persisted source inventory and revisions", async () => {
		const boards = new Map<
			string,
			{ id: string; name: string; source: string }
		>();
		const writes: string[] = [];
		const opened = new Set<string>();
		const backend = {
			appState: {
				createApp: async (_meta: unknown, _bits: unknown, online: boolean) => {
					expect(online).toBe(false);
					return { id: "source" };
				},
			},
			userState: {
				getSettingsProfile: async () => ({}),
				updateProfileApp: async () => {},
			},
			boardState: {
				getBoardSummariesAuthoritative: async () => [...boards.values()],
				upsertBoard: async (app: string, id: string, name: string) => {
					expect(app).toBe("source");
					boards.set(id, { id, name, source: "" });
				},
				applyFlowScript: async (app: string, id: string, source: string) => {
					expect(app).toBe("source");
					expect(opened.has(id)).toBe(true);
					writes.push(id);
					const board = boards.get(id);
					if (board) board.source = source;
					return { commands: [{}], board_commands: [], diagnostics: [] };
				},
				getFlowScriptAuthoritative: async (_app: string, id: string) => {
					opened.add(id);
					return boards.get(id)?.source ?? "";
				},
				checkFlowScriptReconcile: async () => ({
					parse_valid: true,
					reconcile_valid: true,
				}),
			},
		} as unknown as IBackendState;
		const seed = await createRetrievalSeed(backend, "pair");
		expect(writes).toHaveLength(4);
		expect(seed.fixtureHash).toHaveLength(64);
		expect(await readRetrievalSeedHash(backend, seed)).toBe(seed.canonicalHash);
		const first = boards.values().next().value;
		if (!first) throw new Error("No board");
		first.source += '\nfunction extra(): (out: string) { return "changed" }';
		expect(await readRetrievalSeedHash(backend, seed)).not.toBe(
			seed.canonicalHash,
		);
		boards.delete(first.id);
		await expect(readRetrievalSeedHash(backend, seed)).rejects.toThrow(
			"inventory changed",
		);
	});
});
