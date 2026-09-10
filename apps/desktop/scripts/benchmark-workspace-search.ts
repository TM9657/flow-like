import { execFileSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { readFlowScriptSource } from "../../../packages/ui/components/global-chat/read-flowscript-source";
import {
	type WorkspaceScope,
	collectWorkspaceDocuments,
} from "../../../packages/ui/lib/flowpilot/workspace-content";
import { createWorkspaceIndex } from "../../../packages/ui/lib/flowpilot/workspace-ranker";
import {
	createWorkspaceIndex as createBaselineIndex,
	rankWorkspaceDocuments as rankBaselineDocuments,
} from "../../../packages/ui/lib/flowpilot/workspace-ranker-baseline";
import {
	type WorkspaceDocument,
	jsonBytes,
	workspaceRevision,
} from "../../../packages/ui/lib/flowpilot/workspace-resource";
import {
	WorkspaceSearchSession,
	rankWorkspaceDocuments,
	readWorkspaceSymbol,
} from "../../../packages/ui/lib/flowpilot/workspace-search";
import type { IBackendState } from "../../../packages/ui/state/backend-state";

interface Target {
	source_path: string;
	symbol?: string;
	section_id?: string;
}
interface Query {
	id: string;
	query: string;
	category: string;
	targets: Target[];
	reason: string;
}
interface Board {
	path: string;
	app: string;
	source: string;
}
type Hits = ReturnType<typeof rankWorkspaceDocuments>["hits"];

const root = resolve(import.meta.dir, "../../..");
const { values } = parseArgs({
	options: {
		output: {
			type: "string",
			default: "/private/tmp/flowpilot-workspace-benchmark.json",
		},
		queries: {
			type: "string",
			default: "apps/desktop/scripts/workspace-search-queries.json",
		},
		repetitions: { type: "string", default: "20" },
		"quality-only": { type: "boolean", default: false },
	},
	strict: true,
});
const repetitions = Number(values.repetitions);
if (!Number.isInteger(repetitions) || repetitions < 5 || repetitions > 200)
	throw new Error("Use 5 to 200 repetitions.");
const querySource = await readFile(resolve(root, values.queries), "utf8");
const parsedQueries = JSON.parse(querySource);
const queries: Query[] = Array.isArray(parsedQueries)
	? parsedQueries
	: parsedQueries.queries;
if (!Array.isArray(queries) || !queries.length)
	throw new Error("Missing benchmark queries.");

const tracked = execFileSync("git", ["ls-files", "-z"], {
	cwd: root,
	encoding: "utf8",
}).split("\0");
const paths = tracked
	.filter(
		(path) =>
			(path.startsWith("apps/book/examples/") && path.endsWith(".flow")) ||
			(path.startsWith("tests/ast/") &&
				path.endsWith(".flow") &&
				!path.endsWith(".anchored.flow") &&
				!path.endsWith(".wip.flow") &&
				!path.split("/").at(-1)?.startsWith("bug-")) ||
			path === "packages/catalog/tests/fixtures/execution_chain.flowscript",
	)
	.sort();
if (paths.length > 64)
	throw new Error(
		"Corpus exceeds four apps of sixteen boards; revise the benchmark scope explicitly.",
	);
const loadStart = performance.now();
const boards: Board[] = await Promise.all(
	paths.map(async (path, index) => ({
		path,
		app: `benchmark-app-${Math.floor(index / 16) + 1}`,
		source: await readFile(resolve(root, path), "utf8"),
	})),
);
const fixtureLoadMs = performance.now() - loadStart;

function fixtureBackend(delayMs = 0) {
	const stats = {
		inventory_reads: 0,
		source_reads: 0,
		profile_reads: 0,
		source_bytes: 0,
		peak_in_flight: 0,
		backend_wait_ms: 0,
	};
	let active = 0;
	let waitingSince = 0;
	const pause = async () => {
		if (active++ === 0) waitingSince = performance.now();
		stats.peak_in_flight = Math.max(stats.peak_in_flight, active);
		if (delayMs) await new Promise((done) => setTimeout(done, delayMs));
		if (--active === 0)
			stats.backend_wait_ms += performance.now() - waitingSince;
	};
	const readSource = async (app: string, board: string) => {
		stats.source_reads++;
		await pause();
		const found = boards.find(
			(item) => item.app === app && item.path === board,
		);
		if (!found) throw new Error("Unknown benchmark board");
		stats.source_bytes += Buffer.byteLength(found.source);
		return found.source;
	};
	const backend = {
		boardState: {
			getBoardSummariesAuthoritative: async (app: string) => {
				stats.inventory_reads++;
				await pause();
				return boards
					.filter((item) => item.app === app)
					.map((item) => ({ id: item.path }));
			},
			getFlowScriptAuthoritative: readSource,
		},
	} as unknown as IBackendState;
	const scope: WorkspaceScope = {
		getProfileAppIds: async () => {
			stats.profile_reads++;
			return new Set(boards.map((board) => board.app));
		},
	};
	return { backend, scope, stats, readSource };
}

function summary(samples: number[]) {
	const sorted = [...samples].sort((a, b) => a - b);
	const quantile = (fraction: number) =>
		sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)] ?? null;
	return {
		n: sorted.length,
		p50: quantile(0.5),
		p95: quantile(0.95),
		min: sorted[0] ?? null,
		max: sorted.at(-1) ?? null,
	};
}

const coldFixture = fixtureBackend();
const collectStart = performance.now();
const collected = await collectWorkspaceDocuments(
	coldFixture.backend,
	{ kinds: ["workflow", "doc"] },
	coldFixture.scope,
);
const collectMs = performance.now() - collectStart;
if (!collected.coverage.complete)
	throw new Error(`Incomplete corpus: ${JSON.stringify(collected.coverage)}`);
const documents = collected.documents;
const descriptors = documents.map((document) => ({ ...document, content: "" }));
const byId = new Map(
	documents.map((document) => [document.resource_id, document]),
);
function isTarget(document: WorkspaceDocument, target: Target) {
	if (document.target.kind === "workflow")
		return (
			document.target.board_id === target.source_path &&
			document.symbol === target.symbol
		);
	return (
		document.source_path === target.source_path &&
		document.target.id === `doc:${target.source_path}#${target.section_id}`
	);
}
const expected = new Map(
	queries.map((query) => {
		const ids = query.targets.map((target) => {
			const found = documents.find((document) => isTarget(document, target));
			if (!found)
				throw new Error(
					`Unknown frozen target in ${query.id}: ${JSON.stringify(target)}`,
				);
			return found.resource_id;
		});
		return [query.id, new Set(ids)] as const;
	}),
);
const evaluate = (query: Query, hits: Hits) => {
	const gold = expected.get(query.id);
	if (!gold) throw new Error("Missing target set");
	const found = hits.findIndex((hit) => gold.has(hit.resource_id));
	return {
		expected_rank: found < 0 ? null : found + 1,
		returned: hits.length,
		top: hits.slice(0, 5).map((hit) => ({
			source_path: byId.get(hit.resource_id)?.source_path ?? hit.board_id,
			symbol: hit.symbol,
			title: hit.title,
			score: hit.score,
		})),
	};
};
const quality = queries.map((query) => ({
	...query,
	baseline: evaluate(query, rankBaselineDocuments(documents, query.query).hits),
	descriptors: evaluate(
		query,
		rankWorkspaceDocuments(descriptors, query.query).hits,
	),
	full_text: evaluate(
		query,
		rankWorkspaceDocuments(documents, query.query).hits,
	),
}));
type QualityRow = (typeof quality)[number];
function qualitySummary(
	rows: QualityRow[],
	method: "descriptors" | "full_text" | "baseline",
) {
	const positive = rows.filter((row) => row.targets.length);
	const absent = rows.filter((row) => !row.targets.length);
	const hit = (rank: number | null, k: number) => rank !== null && rank <= k;
	return {
		positive_queries: positive.length,
		hit_at_1: positive.filter((row) => hit(row[method].expected_rank, 1))
			.length,
		hit_at_5: positive.filter((row) => hit(row[method].expected_rank, 5))
			.length,
		hit_at_10: positive.filter((row) => hit(row[method].expected_rank, 10))
			.length,
		mrr: positive.length
			? positive.reduce(
					(sum, row) =>
						sum +
						(row[method].expected_rank
							? 1 / (row[method].expected_rank as number)
							: 0),
					0,
				) / positive.length
			: null,
		absent_queries: absent.length,
		absent_queries_with_no_hits: absent.filter(
			(row) => row[method].returned === 0,
		).length,
	};
}

const timings = {
	baseline: [] as number[],
	descriptors: [] as number[],
	full_text: [] as number[],
};
const retainedIndex = createWorkspaceIndex(documents);
const retainedQueryTimings: number[] = [];
// Alternate order and rotate queries so warmup and scheduling do not always favor one view.
for (
	let round = values["quality-only"] ? repetitions : -2;
	round < repetitions;
	round++
) {
	for (let index = 0; index < queries.length; index++) {
		const query = queries[(index + Math.max(0, round)) % queries.length];
		const order = ["descriptors", "baseline", "full_text"] as const;
		const pivot = (Math.max(0, round) + index) % order.length;
		const methods = [...order.slice(pivot), ...order.slice(0, pivot)];
		for (const method of methods) {
			const start = performance.now();
			(method === "baseline" ? rankBaselineDocuments : rankWorkspaceDocuments)(
				method === "descriptors" ? descriptors : documents,
				query.query,
			);
			if (round >= 0) timings[method].push(performance.now() - start);
		}
		const start = performance.now();
		retainedIndex.search(query.query);
		if (round >= 0) retainedQueryTimings.push(performance.now() - start);
	}
}

const readComparisons = [];
const uniqueTargets = [
	...new Set(queries.flatMap((query) => [...(expected.get(query.id) ?? [])])),
]
	.map((id) => byId.get(id))
	.filter(
		(document): document is WorkspaceDocument =>
			document?.target.kind === "workflow",
	);
for (const document of values["quality-only"] ? [] : uniqueTargets) {
	const fixture = fixtureBackend();
	const oldTimings: number[] = [];
	const exactTimings: number[] = [];
	let oldResult: Record<string, unknown> = {};
	let exactResult: Record<string, unknown> = {};
	for (let round = -2; round < repetitions; round++) {
		for (const method of round % 2 ? ["old", "exact"] : ["exact", "old"]) {
			const start = performance.now();
			if (method === "old") {
				oldResult = await readFlowScriptSource(
					{
						appId: document.target.app_id ?? "",
						boardId: document.target.board_id ?? "",
						locator: document.symbol,
					},
					{
						getProfileAppIds: fixture.scope.getProfileAppIds,
						getFlowScript: fixture.readSource,
					},
				);
				if (round >= 0) oldTimings.push(performance.now() - start);
			} else {
				exactResult = await readWorkspaceSymbol(
					fixture.backend,
					{
						resource_id: document.resource_id,
						revision: document.revision,
					},
					fixture.scope,
				);
				if (round >= 0) exactTimings.push(performance.now() - start);
			}
		}
	}
	if (oldResult.status !== "ok" || exactResult.status !== "ok")
		throw new Error("Known-target read failed");
	readComparisons.push({
		source_path: document.target.board_id,
		symbol: document.symbol,
		old_source_read: {
			reply_bytes: jsonBytes(oldResult),
			content_bytes: Buffer.byteLength(String(oldResult.source)),
			complete_target_in_first_read: String(oldResult.source).includes(
				document.content,
			),
			cpu_ms: summary(oldTimings),
		},
		exact_symbol_read: {
			reply_bytes: jsonBytes(exactResult),
			content_bytes: Buffer.byteLength(String(exactResult.content)),
			complete_target_in_first_read: exactResult.next_offset === null,
			cpu_ms: summary(exactTimings),
		},
	});
}

const selected =
	uniqueTargets.find((document) => document.target.board_id?.includes("t4-")) ??
	uniqueTargets[0];
if (!selected) throw new Error("Benchmark needs a workflow target");
const selectedQuery = queries.find((query) =>
	expected.get(query.id)?.has(selected.resource_id),
);
if (!selectedQuery) throw new Error("Missing selected query");
const serviceCosts = [];
interface ServiceObservation {
	elapsed_ms: number;
	reply_bytes: number;
	inventory_reads: number;
	source_reads: number;
	profile_reads: number;
	source_bytes: number;
	backend_wait_ms: number;
	peak_in_flight: number;
	cache: unknown;
}
for (const delayMs of values["quality-only"] ? [] : [0, 25, 100]) {
	for (const scopeName of ["all_profile_apps", "one_app", "one_board"]) {
		for (const method of ["baseline", "improved"]) {
			const cold: ServiceObservation[] = [];
			const followup: ServiceObservation[] = [];
			for (let round = 0; round < 5; round++) {
				const fixture = fixtureBackend(delayMs);
				const session = new WorkspaceSearchSession(
					Date.now,
					method === "baseline"
						? { createIndex: createBaselineIndex, cacheEnabled: false }
						: {},
				);
				for (const phase of ["cold", "followup"]) {
					const prior = { ...fixture.stats };
					const start = performance.now();
					const result = await session.search(
						fixture.backend,
						{
							query: phase === "cold" ? selected.symbol : selectedQuery.query,
							kinds: ["workflow", "doc"],
							limit: 5,
							...(scopeName === "all_profile_apps"
								? {}
								: { app_id: selected.target.app_id }),
							...(scopeName === "one_board"
								? { board_id: selected.target.board_id }
								: {}),
						},
						fixture.scope,
					);
					const elapsed = performance.now() - start;
					if (result.status !== "ok")
						throw new Error(`Search failed: ${JSON.stringify(result)}`);
					const observations = phase === "cold" ? cold : followup;
					observations.push({
						elapsed_ms: elapsed,
						reply_bytes: jsonBytes(result),
						inventory_reads:
							fixture.stats.inventory_reads - prior.inventory_reads,
						source_reads: fixture.stats.source_reads - prior.source_reads,
						profile_reads: fixture.stats.profile_reads - prior.profile_reads,
						source_bytes: fixture.stats.source_bytes - prior.source_bytes,
						backend_wait_ms:
							fixture.stats.backend_wait_ms - prior.backend_wait_ms,
						peak_in_flight: fixture.stats.peak_in_flight,
						cache: (result.freshness as Record<string, unknown> | undefined)
							?.cache,
					});
				}
			}
			for (const [phase, observations] of [
				["cold", cold],
				["followup", followup],
			] as const)
				serviceCosts.push({
					method,
					phase,
					scope: scopeName,
					simulated_backend_delay_ms: delayMs,
					elapsed_ms: summary(observations.map((item) => item.elapsed_ms)),
					observations,
				});
		}
	}
}

const fingerprintPaths = [
	"packages/ui/lib/flowpilot/workspace-search.ts",
	"packages/ui/lib/flowpilot/workspace-ranker.ts",
	"packages/ui/lib/flowpilot/workspace-ranker-baseline.ts",
	"packages/ui/lib/flowpilot/workspace-content.ts",
	"packages/ui/lib/flowpilot/workspace-symbols.ts",
	"packages/ui/lib/flowpilot/workspace-resource.ts",
	"packages/ui/lib/flowpilot/generated/workspace-docs.json",
	"packages/ui/lib/search-index.ts",
	"packages/ui/components/global-chat/read-flowscript-source.ts",
	"apps/desktop/scripts/benchmark-workspace-search.ts",
];
const fingerprints = Object.fromEntries(
	await Promise.all(
		fingerprintPaths.map(async (path) => [
			path,
			await workspaceRevision(await readFile(resolve(root, path), "utf8")),
		]),
	),
);
const report = {
	schema_version: 2,
	generated_at: new Date().toISOString(),
	environment: {
		runtime: `Bun ${Bun.version}`,
		platform: process.platform,
		arch: process.arch,
		repetitions,
		quality_only: values["quality-only"],
		git_head: execFileSync("git", ["rev-parse", "HEAD"], {
			cwd: root,
			encoding: "utf8",
		}).trim(),
		query_sha256: await workspaceRevision(querySource),
		query_semantic_sha256: await workspaceRevision(JSON.stringify(queries)),
		implementation_sha256: fingerprints,
	},
	methodology: {
		quality:
			"Frozen independently authored queries over committed FlowScript and bundled docs. Expected-target hit rates use sparse labels, not exhaustive relevance judgments.",
		descriptors:
			"Same production ranker with content removed; names/signatures/titles retained. This is an ablation, not the old agent harness.",
		read_comparison:
			"Actual previous read_flowscript_source vs read_symbol, both given the known target. This measures payload and local processing, not discovery success.",
		timing:
			"Two warmup rounds, rotating method order and queries. Index-build-and-rank durations rebuild the index. retained_index_query_ms measures only the improved cached index search. Backend fixture serves real source files from memory.",
		service_cost:
			"Production collection and search with instrumented in-memory backend. Delay 0 measures local work; 25/100 ms are injected per inventory/source request, not observed network latency. Five repeats per condition; small-sample p95 is the maximum.",
		limits:
			"No model generation, hosted API, transport, token usage or user acceptance runs. Comparisons do not establish end-to-end app-build speed or quality. Corpus excludes anchored duplicate variants, bug and work-in-progress fixtures.",
	},
	corpus: {
		boards: boards.length,
		apps: new Set(boards.map((board) => board.app)).size,
		workflow_symbols: documents.filter(
			(document) => document.target.kind === "workflow",
		).length,
		doc_sections: documents.filter((document) => document.target.kind === "doc")
			.length,
		source_bytes: boards.reduce(
			(sum, board) => sum + Buffer.byteLength(board.source),
			0,
		),
		fixture_load_ms: fixtureLoadMs,
		cold_collect_ms: collectMs,
		cold_collect_backend: coldFixture.stats,
		coverage: collected.coverage,
		files: await Promise.all(
			boards.map(async (board) => ({
				path: board.path,
				sha256: await workspaceRevision(board.source),
			})),
		),
	},
	quality: {
		baseline: qualitySummary(quality, "baseline"),
		descriptors: qualitySummary(quality, "descriptors"),
		full_text: qualitySummary(quality, "full_text"),
		by_category: Object.fromEntries(
			[...new Set(queries.map((query) => query.category))].map((category) => [
				category,
				{
					descriptors: qualitySummary(
						quality.filter((row) => row.category === category),
						"descriptors",
					),
					full_text: qualitySummary(
						quality.filter((row) => row.category === category),
						"full_text",
					),
				},
			]),
		),
		cases: quality,
	},
	retained_index_query_ms: summary(retainedQueryTimings),
	index_build_and_rank_ms: {
		baseline: summary(timings.baseline),
		descriptors: summary(timings.descriptors),
		full_text: summary(timings.full_text),
	},
	known_target_reads: {
		cases: readComparisons,
		old_reply_bytes: summary(
			readComparisons.map((item) => item.old_source_read.reply_bytes),
		),
		exact_reply_bytes: summary(
			readComparisons.map((item) => item.exact_symbol_read.reply_bytes),
		),
		old_total_reply_bytes: readComparisons.reduce(
			(sum, item) => sum + item.old_source_read.reply_bytes,
			0,
		),
		exact_total_reply_bytes: readComparisons.reduce(
			(sum, item) => sum + item.exact_symbol_read.reply_bytes,
			0,
		),
	},
	service_costs: {
		query: selectedQuery.query,
		source_path: selected.target.board_id,
		cases: serviceCosts,
	},
};
const output = resolve(root, values.output);
await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
console.log(
	JSON.stringify(
		{
			output,
			corpus: {
				boards: report.corpus.boards,
				symbols: report.corpus.workflow_symbols,
				docs: report.corpus.doc_sections,
			},
			quality: {
				baseline: report.quality.baseline,
				descriptors: report.quality.descriptors,
				full_text: report.quality.full_text,
			},
			timing: report.index_build_and_rank_ms,
			reads: {
				old_bytes: report.known_target_reads.old_total_reply_bytes,
				exact_bytes: report.known_target_reads.exact_total_reply_bytes,
			},
			service: serviceCosts.map(
				({
					method,
					phase,
					scope,
					simulated_backend_delay_ms,
					elapsed_ms,
					observations,
				}) => ({
					method,
					phase,
					scope,
					simulated_backend_delay_ms,
					elapsed_ms,
					reads: observations[0].source_reads,
				}),
			),
		},
		null,
		2,
	),
);
