import { workspaceRevision } from "@flow-like/flow-like-ui/lib/flowpilot/workspace-resource";
import { extractFlowScriptWorkspaceSymbols } from "@flow-like/flow-like-ui/lib/flowpilot/workspace-symbols";
import {
	IExecutionStage,
	ILogLevel,
} from "@flow-like/flow-like-ui/lib/schema/flow/board";
import { nowSystemTime } from "@flow-like/flow-like-ui/lib/time/now";
import type { IBackendState } from "@flow-like/flow-like-ui/state/backend-state";
import { type RetrievalSeed, retrievalSeedHash } from "./retrieval-comparison";

export const RETRIEVAL_SEED_SOURCES = [
	{
		name: "Intake handling",
		source: `use string::{ containsAny, toLower }
use log::{ info }
function routeRecord(summary: string): (queue: string, minutes: int) {
 const text = summary.toLower()
 const { contains: interruption } = text.containsAny({ substrings: ["outage", "service interruption", "cannot log in"], ignoreCase: true })
 const { contains: billing } = text.containsAny({ substrings: ["refund", "charged twice", "billing"], ignoreCase: true })
 let queue = "general-desk"
 let minutes = 2880
 if (interruption) { queue = "cobalt-response"; minutes = 17 }
 else if (billing) { queue = "amber-review"; minutes = 731 }
 return queue, minutes
}
eventsSimple sampleIntake() {
 const { queue, minutes } = routeRecord("service interruption")
 info({ message: \`\${queue}:\${minutes}\`, toast: false })
}`,
	},
	...[
		["Stock handling", "stock", "warehouse-hold"],
		["Travel handling", "travel", "travel-review"],
		["Archive handling", "archive", "retention-hold"],
	].map(([name, term, result]) => ({
		name,
		source: `use log::{ info }
function classifyRecord(summary: string): (queue: string) { return "${result}" }
eventsSimple sampleRecord() {
 const queue = classifyRecord("${term}")
 info({ message: queue, toast: false })
}`,
	})),
] as const;

export async function readRetrievalSeedHash(
	backend: IBackendState,
	seed: RetrievalSeed,
) {
	const summaries = await backend.boardState.getBoardSummariesAuthoritative(
		seed.appId,
	);
	if (
		summaries.length !== seed.boards.length ||
		seed.boards.some((board) => !summaries.some((item) => item.id === board.id))
	) {
		throw new Error("Retrieval source board inventory changed.");
	}
	const sources = await Promise.all(
		seed.boards.map(async (board) => ({
			name: board.name,
			source: await backend.boardState.getFlowScriptAuthoritative(
				seed.appId,
				board.id,
				undefined,
				true,
			),
		})),
	);
	return retrievalSeedHash(sources);
}

export async function createRetrievalSeed(
	backend: IBackendState,
	pairId: string,
): Promise<RetrievalSeed> {
	const time = nowSystemTime();
	const app = await backend.appState.createApp(
		{
			name: `Retrieval source ${pairId}`,
			description:
				"Local read-only source fixture for paired FlowPilot retrieval evaluation.",
			tags: [],
			use_case: "",
			created_at: time,
			updated_at: time,
			preview_media: [],
		},
		[],
		false,
	);
	const profile = await backend.userState.getSettingsProfile();
	await backend.userState.updateProfileApp(
		profile,
		{ app_id: app.id, favorite: false, pinned: false },
		"Upsert",
	);
	const initial = await backend.boardState.getBoardSummariesAuthoritative(
		app.id,
	);
	if (initial.length > 1)
		throw new Error("New retrieval fixture unexpectedly has multiple boards.");
	const boards: RetrievalSeed["boards"] = [];
	const canonical: { name: string; source: string }[] = [];
	for (const [index, fixture] of RETRIEVAL_SEED_SOURCES.entries()) {
		const boardId =
			index === 0 && initial[0] ? initial[0].id : crypto.randomUUID();
		await backend.boardState.upsertBoard(
			app.id,
			boardId,
			fixture.name,
			"Retrieval evaluation source",
			ILogLevel.Info,
			IExecutionStage.Dev,
		);
		// Upsert persists metadata without registering a native board handle. Apply requires
		// that handle; the canonical read opens the persisted board and registers it.
		await backend.boardState.getFlowScriptAuthoritative(
			app.id,
			boardId,
			undefined,
			true,
		);
		const applied = await backend.boardState.applyFlowScript(
			app.id,
			boardId,
			fixture.source,
			undefined,
			undefined,
			false,
			"editor",
		);
		if (!applied.commands.length || applied.diagnostics.length) {
			throw new Error(
				`Seed ${fixture.name} did not apply successfully: ${JSON.stringify(applied.diagnostics)}`,
			);
		}
		const source = await backend.boardState.getFlowScriptAuthoritative(
			app.id,
			boardId,
			undefined,
			true,
		);
		const expectedSymbols = extractFlowScriptWorkspaceSymbols(fixture.source);
		const actualSymbols = extractFlowScriptWorkspaceSymbols(source);
		if (
			!expectedSymbols.complete ||
			!actualSymbols.complete ||
			!expectedSymbols.symbols.length ||
			expectedSymbols.symbols.some(
				(expected) =>
					!actualSymbols.symbols.some(
						(actual) =>
							actual.kind === expected.kind &&
							actual.qualifiedName === expected.qualifiedName,
					),
			)
		) {
			throw new Error(
				`Seed ${fixture.name} canonical source lacks its expected helper and Event declarations.`,
			);
		}
		const check = await backend.boardState.checkFlowScriptReconcile?.(
			app.id,
			boardId,
			source,
		);
		if (!check?.parse_valid || !check.reconcile_valid)
			throw new Error(
				`Seed ${fixture.name} did not pass canonical reconciliation.`,
			);
		canonical.push({ name: fixture.name, source });
		boards.push({
			id: boardId,
			name: fixture.name,
			revision: await workspaceRevision(source),
		});
	}
	return {
		appId: app.id,
		fixtureHash: await retrievalSeedHash(RETRIEVAL_SEED_SOURCES),
		canonicalHash: await retrievalSeedHash(canonical),
		boards,
	};
}
