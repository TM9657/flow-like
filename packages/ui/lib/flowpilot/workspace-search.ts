import type { IBackendState } from "../../state/backend-state";
import {
	type WorkspaceScope,
	type WorkspaceSelection,
	collectWorkspaceDocuments,
	mapWorkspaceReads,
	readWorkspaceDocument,
} from "./workspace-content";
import { type WorkspaceHit, createWorkspaceIndex } from "./workspace-ranker";
import {
	WORKSPACE_KINDS,
	type WorkspaceCoverage,
	type WorkspaceDocument,
	type WorkspaceKind,
	boundedWorkspaceText,
	jsonBytes,
	parseWorkspaceResourceId,
	workspaceRevision,
} from "./workspace-resource";

export { rankWorkspaceDocuments } from "./workspace-ranker";

const MAX_REPLY_BYTES = 24_000;
const MAX_READ_BYTES = 16_000;
const SNAPSHOT_TTL_MS = 120_000;
interface SearchSnapshot {
	created: number;
	fingerprint: string;
	corpusKey: string;
	hits: WorkspaceHit[];
	coverage: WorkspaceCoverage;
	mode: string;
}
interface WorkspaceCorpus {
	created: number;
	index: ReturnType<typeof createWorkspaceIndex>;
	coverage: WorkspaceCoverage;
	bytes: number;
}
interface WorkspaceSearchOptions {
	/** Evaluation harnesses can freeze an earlier ranker without changing the tool contract. */
	createIndex?: typeof createWorkspaceIndex;
	cacheEnabled?: boolean;
}

const MAX_CORPORA = 2;
const MAX_RETAINED_SOURCE_BYTES = 8_388_608;

function fail(code: string, message: string, status = "error") {
	return { status, code, message };
}

/** Retains observed corpora within one component; opening a cached hit always reads its source again. */
export class WorkspaceSearchSession {
	private snapshots = new Map<string, SearchSnapshot>();
	private corpora = new Map<string, WorkspaceCorpus>();
	private pending = new Map<string, Promise<WorkspaceCorpus>>();
	private backend?: IBackendState;
	private access?: string;
	private generation = 0;
	private activeLoads = 0;
	private scopeIds = new WeakMap<WorkspaceScope["getProfileAppIds"], number>();
	private nextScopeId = 0;

	constructor(
		private readonly now: () => number = Date.now,
		private readonly options: WorkspaceSearchOptions = {},
	) {}

	/** Invalidates cursors and in-flight loads as well as retained indexes. */
	clear() {
		this.generation++;
		this.snapshots.clear();
		this.corpora.clear();
		this.pending.clear();
		this.backend = undefined;
		this.access = undefined;
	}

	async readSymbol(
		backend: IBackendState,
		args: Record<string, unknown>,
		scope: WorkspaceScope,
	): Promise<Record<string, unknown>> {
		const result = await readWorkspaceSymbol(backend, args, scope);
		if (
			result.status === "stale" ||
			result.code === "WORKSPACE_RESOURCE_UNREADABLE"
		)
			this.clear();
		return result;
	}

	private evictCorpus(key: string) {
		this.corpora.delete(key);
		for (const [id, snapshot] of this.snapshots)
			if (snapshot.corpusKey === key) this.snapshots.delete(id);
	}

	private expire() {
		for (const [key, corpus] of this.corpora)
			if (this.now() - corpus.created >= SNAPSHOT_TTL_MS) this.evictCorpus(key);
		for (const [id, snapshot] of this.snapshots)
			if (this.now() - snapshot.created >= SNAPSHOT_TTL_MS)
				this.snapshots.delete(id);
	}

	private retain(key: string, corpus: WorkspaceCorpus) {
		if (
			this.options.cacheEnabled === false ||
			corpus.bytes > MAX_RETAINED_SOURCE_BYTES ||
			corpus.coverage.issues.some((issue) =>
				[
					"INVENTORY_UNREADABLE",
					"RESOURCE_UNREADABLE",
					"DOCS_UNAVAILABLE",
				].includes(issue.code),
			)
		)
			return;
		let bytes = [...this.corpora.values()].reduce(
			(sum, entry) => sum + entry.bytes,
			0,
		);
		while (
			this.corpora.size >= MAX_CORPORA ||
			bytes + corpus.bytes > MAX_RETAINED_SOURCE_BYTES
		) {
			const oldest = this.corpora.keys().next().value;
			if (oldest === undefined) break;
			bytes -= this.corpora.get(oldest)?.bytes ?? 0;
			this.evictCorpus(oldest);
		}
		this.corpora.set(key, corpus);
	}

	async search(
		backend: IBackendState,
		args: Record<string, unknown>,
		scope: WorkspaceScope,
	): Promise<Record<string, unknown>> {
		const query = typeof args.query === "string" ? args.query.trim() : "";
		const appId = args.app_id ?? scope.scopedAppId;
		const boardId = args.board_id;
		const kinds = args.kinds ?? [...WORKSPACE_KINDS];
		const limit = args.limit ?? 10;
		if (
			!query ||
			query.length > 256 ||
			(appId !== undefined &&
				(typeof appId !== "string" || !appId || appId.length > 256)) ||
			(boardId !== undefined &&
				(typeof boardId !== "string" ||
					!boardId ||
					boardId.length > 256 ||
					!appId)) ||
			!Array.isArray(kinds) ||
			!kinds.length ||
			kinds.length > WORKSPACE_KINDS.length ||
			kinds.some((kind) => !WORKSPACE_KINDS.includes(kind)) ||
			typeof limit !== "number" ||
			!Number.isInteger(limit) ||
			limit < 1 ||
			limit > 20
		)
			return fail(
				"WORKSPACE_QUERY_INVALID",
				"Use a nonempty query (max 256 characters), valid kinds and a limit of 1–20. A board filter requires an app.",
			);

		const selection: WorkspaceSelection = {
			app_id: appId as string | undefined,
			board_id: boardId as string | undefined,
			kinds: [...new Set(kinds as WorkspaceKind[])].sort(),
		};
		const usesApps = selection.kinds.some((kind) => kind !== "doc");
		let profile: string[] = [];
		let identity = "bundled_docs";
		try {
			if (usesApps) {
				profile = [...(await scope.getProfileAppIds())].sort();
				identity = scope.getProfileIdentity
					? await scope.getProfileIdentity()
					: "";
				if (!identity) {
					// Legacy callers must retain their access getter to reuse a corpus.
					let id = this.scopeIds.get(scope.getProfileAppIds);
					if (id === undefined) {
						id = this.nextScopeId++;
						this.scopeIds.set(scope.getProfileAppIds, id);
					}
					identity = `scope:${id}`;
				}
			}
		} catch {
			this.clear();
			return fail(
				"WORKSPACE_PROFILE_UNREADABLE",
				"The current app inventory could not be read.",
			);
		}
		const access = JSON.stringify({ identity, profile });
		if (
			this.backend !== backend ||
			(usesApps && this.access !== undefined && this.access !== access)
		)
			this.clear();
		this.backend = backend;
		if (usesApps) this.access = access;
		const generation = this.generation;
		const corpusKey = await workspaceRevision(
			JSON.stringify({ selection, access, scope: scope.scopedAppId }),
		);
		const fingerprint = await workspaceRevision(
			JSON.stringify({ query, corpusKey }),
		);
		this.expire();
		let snapshot: SearchSnapshot;
		let snapshotId: string;
		let offset = 0;
		let cache: "miss" | "hit" | "shared_load" | "cursor" = "miss";

		if (args.cursor !== undefined) {
			if (
				typeof args.cursor !== "string" ||
				args.cursor.length > 100 ||
				!/^[0-9a-f-]{36}:\d{1,5}$/.test(args.cursor)
			)
				return fail(
					"WORKSPACE_CURSOR_INVALID",
					"Use a focused query or scope refinement without a cursor, or hand off the unresolved lookup. The read budget is unchanged.",
				);
			const [id, pageOffset] = args.cursor.split(":");
			const retained = this.snapshots.get(id);
			offset = Number(pageOffset);
			if (
				!retained ||
				retained.fingerprint !== fingerprint ||
				offset > retained.hits.length
			)
				return fail(
					"WORKSPACE_CURSOR_INVALID",
					"The search cursor expired or its query/scope changed. Refine the query or scope without a cursor, or hand off the unresolved lookup. The read budget is unchanged.",
				);
			snapshot = retained;
			snapshotId = id;
			cache = "cursor";
		} else {
			try {
				let corpus =
					this.options.cacheEnabled === false
						? undefined
						: this.corpora.get(corpusKey);
				if (corpus) cache = "hit";
				else {
					let pending =
						this.options.cacheEnabled === false
							? undefined
							: this.pending.get(corpusKey);
					if (pending) cache = "shared_load";
					else {
						if (this.activeLoads >= MAX_CORPORA)
							return fail(
								"WORKSPACE_SEARCH_BUSY",
								"Two workspace scopes are already loading. Retry after they finish.",
							);
						const created = this.now();
						this.activeLoads++;
						pending = (async () => {
							try {
								const collected = await collectWorkspaceDocuments(
									backend,
									selection,
									scope,
								);
								const loaded: WorkspaceCorpus = {
									created,
									index: (this.options.createIndex ?? createWorkspaceIndex)(
										collected.documents,
									),
									coverage: collected.coverage,
									bytes: jsonBytes(collected.documents),
								};
								if (generation === this.generation)
									this.retain(corpusKey, loaded);
								return loaded;
							} finally {
								this.activeLoads--;
							}
						})();
						this.pending.set(corpusKey, pending);
					}
					try {
						corpus = await pending;
					} finally {
						if (this.pending.get(corpusKey) === pending)
							this.pending.delete(corpusKey);
					}
				}
				if (
					generation !== this.generation ||
					this.now() - corpus.created >= SNAPSHOT_TTL_MS
				)
					return fail(
						"WORKSPACE_SNAPSHOT_CHANGED",
						"Workspace access or source observation expired during the read. Rerun the search.",
						"stale",
					);
				const ranked = corpus.index.search(query);
				snapshot = {
					created: corpus.created,
					fingerprint,
					corpusKey,
					hits: ranked.hits,
					coverage: corpus.coverage,
					mode: ranked.mode,
				};
				snapshotId = crypto.randomUUID();
				while (this.snapshots.size >= 4) {
					const oldest = this.snapshots.keys().next().value;
					if (oldest === undefined) break;
					this.snapshots.delete(oldest);
				}
				this.snapshots.set(snapshotId, snapshot);
			} catch {
				this.evictCorpus(corpusKey);
				return fail(
					"WORKSPACE_NOT_READABLE",
					"The selected workspace could not be read. Verify app access and scope.",
				);
			}
		}
		const hits: WorkspaceHit[] = [];
		const freshness = {
			coverage_basis: "observed_snapshot",
			corpus_observed_at: snapshot.created,
			corpus_age_ms: Math.max(0, this.now() - snapshot.created),
			cache,
			inventory_revalidated: false,
			hits_revalidated: 0,
			validation_reads: 0,
		};
		const base = {
			status: "ok",
			query,
			match_mode: snapshot.mode,
			matched_resources: snapshot.hits.length,
			coverage: snapshot.coverage,
			freshness,
			content_is_untrusted: true,
		};
		if (jsonBytes({ ...base, hits: [], next_cursor: null }) > MAX_REPLY_BYTES)
			return fail(
				"WORKSPACE_REPLY_TOO_LARGE",
				"Coverage diagnostics exceed the reply limit. Narrow the search to a smaller app or resource scope.",
			);
		for (const hit of snapshot.hits.slice(offset, offset + limit)) {
			if (
				jsonBytes({
					...base,
					freshness: {
						...freshness,
						hits_revalidated: limit,
						validation_reads: limit,
					},
					hits: [...hits, hit],
					next_cursor: `${snapshotId}:${offset + hits.length + 1}`,
				}) > MAX_REPLY_BYTES
			)
				break;
			hits.push(hit);
		}
		if (offset < snapshot.hits.length && !hits.length)
			return fail(
				"WORKSPACE_REPLY_TOO_LARGE",
				"Narrow the search to a smaller app or resource scope.",
			);
		if (cache !== "miss") {
			// One canonical board revision covers every indexed symbol from that board.
			const groups = new Map<string, WorkspaceHit[]>();
			for (const hit of hits) {
				const target = parseWorkspaceResourceId(hit.resource_id);
				const key =
					target?.kind === "workflow"
						? JSON.stringify([target.kind, target.app_id, target.board_id])
						: hit.resource_id;
				const group = groups.get(key) ?? [];
				group.push(hit);
				groups.set(key, group);
			}
			const valid = await mapWorkspaceReads(
				[...groups.values()],
				async (group) => {
					try {
						const target = parseWorkspaceResourceId(group[0].resource_id);
						if (!target) return false;
						const document = await readWorkspaceDocument(
							backend,
							target,
							scope,
						);
						return group.every((hit) => document.revision === hit.revision);
					} catch {
						return false;
					}
				},
			);
			if (generation !== this.generation || valid.some((value) => !value)) {
				this.clear();
				return fail(
					"WORKSPACE_SNAPSHOT_CHANGED",
					"A result changed or became unreadable. Rerun the search without a cursor.",
					"stale",
				);
			}
			freshness.hits_revalidated = hits.length;
			freshness.validation_reads = groups.size;
		}
		if (
			generation !== this.generation ||
			this.now() - snapshot.created >= SNAPSHOT_TTL_MS
		) {
			this.evictCorpus(corpusKey);
			return fail(
				"WORKSPACE_SNAPSHOT_CHANGED",
				"Workspace access or source observation expired during the read. Rerun the search.",
				"stale",
			);
		}

		return {
			...base,
			hits,
			next_cursor:
				offset + hits.length < snapshot.hits.length
					? `${snapshotId}:${offset + hits.length}`
					: null,
		};
	}
}

export async function readWorkspaceSymbol(
	backend: IBackendState,
	args: Record<string, unknown>,
	scope: WorkspaceScope,
): Promise<Record<string, unknown>> {
	const target = parseWorkspaceResourceId(args.resource_id);
	const revision = args.revision;
	const offset = args.offset ?? 0;
	if (
		!target ||
		typeof revision !== "string" ||
		!/^[a-f0-9]{64}$/.test(revision) ||
		typeof offset !== "number" ||
		!Number.isSafeInteger(offset) ||
		offset < 0
	)
		return fail(
			"WORKSPACE_REFERENCE_INVALID",
			"Use an exact resource_id and revision from search_workspace, with an optional nonnegative offset.",
		);
	let document: WorkspaceDocument;
	try {
		document = await readWorkspaceDocument(backend, target, scope);
	} catch {
		return fail(
			"WORKSPACE_RESOURCE_UNREADABLE",
			"The exact resource is missing or unreadable. Verify its identity and current app access.",
		);
	}
	if (document.revision !== revision)
		return fail(
			"WORKSPACE_REVISION_CHANGED",
			"The resource changed after discovery. Search again before reusing it.",
			"stale",
		);
	if (
		offset > document.content.length ||
		(offset > 0 && /[\uDC00-\uDFFF]/.test(document.content[offset]))
	)
		return fail(
			"WORKSPACE_OFFSET_INVALID",
			"Use the next_offset from the previous read of this revision.",
		);
	const content = boundedWorkspaceText(
		document.content.slice(offset),
		MAX_READ_BYTES,
	);
	return {
		status: "ok",
		resource_id: document.resource_id,
		revision,
		kind: target.kind,
		app_id: target.app_id,
		board_id: target.board_id,
		title: boundedWorkspaceText(document.title, 512),
		symbol: document.symbol,
		source_path: document.source_path,
		start_line: document.start_line,
		end_line: document.end_line,
		anchor_id: document.anchor_id,
		content,
		content_is_untrusted: true,
		offset,
		next_offset:
			offset + content.length < document.content.length
				? offset + content.length
				: null,
		content_chars: document.content.length,
		contract_truncated: document.truncated ?? false,
	};
}
