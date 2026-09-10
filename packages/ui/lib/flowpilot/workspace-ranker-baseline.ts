// Frozen September 9, 2026 retrieval baseline for controlled development comparisons.
// Keep AND matching and per-query index construction unchanged.
import MiniSearch from "minisearch";
import { tokenizeSearchText } from "../search-index";
import {
	type WorkspaceDocument,
	type WorkspaceKind,
	boundedWorkspaceText,
} from "./workspace-resource";

const STOP_WORDS = new Set([
	"a",
	"an",
	"the",
	"find",
	"existing",
	"which",
	"where",
	"for",
	"with",
	"and",
	"to",
	"of",
	"in",
]);

export interface WorkspaceHit {
	resource_id: string;
	revision: string;
	kind: WorkspaceKind;
	app_id?: string;
	board_id?: string;
	title: string;
	symbol?: string;
	signature?: string;
	source_path?: string;
	start_line?: number;
	end_line?: number;
	anchor_id?: string;
	snippet: string;
	matched_fields: string[];
	score: number;
}
function snippet(content: string, terms: string[]): string {
	const lower = content.toLowerCase();
	const positions = terms
		.map((term) => lower.indexOf(term))
		.filter((position) => position >= 0);
	let start = Math.max(
		0,
		(positions.length ? Math.min(...positions) : 0) - 100,
	);
	if (start && /[\uDC00-\uDFFF]/.test(content[start])) start--;
	return `${start ? "…" : ""}${boundedWorkspaceText(content.slice(start), 600)}`;
}

/** An in-memory full-text index. Results are evidence to read, never an authorization to call a node. */
export function rankWorkspaceDocuments(
	documents: WorkspaceDocument[],
	query: string,
): { hits: WorkspaceHit[]; mode: string } {
	const terms = [
		...new Set(
			tokenizeSearchText(query)
				.map((term) => term.toLowerCase())
				.filter((term) => !STOP_WORDS.has(term)),
		),
	];
	if (!terms.length) return { hits: [], mode: "no_meaningful_terms" };
	const index = new MiniSearch({
		idField: "resource_id",
		fields: ["title", "symbol", "signature", "content"],
		tokenize: tokenizeSearchText,
		searchOptions: {
			combineWith: "AND",
			prefix: true,
			boost: { symbol: 4, title: 3, signature: 2 },
		},
	});
	const unique = [
		...new Map(
			documents.map((document) => [document.resource_id, document]),
		).values(),
	];
	index.addAll(unique);
	const byId = new Map(
		unique.map((document) => [document.resource_id, document]),
	);
	let results = index.search(terms.join(" "));
	let mode = "prefix";
	if (!results.length) {
		results = index.search(terms.join(" "), { fuzzy: 0.2 });
		mode = "fuzzy";
	}
	results.sort(
		(a, b) => b.score - a.score || String(a.id).localeCompare(String(b.id)),
	);
	const hits = results.map((result): WorkspaceHit => {
		const document = byId.get(String(result.id));
		if (!document) throw new Error("WORKSPACE_INDEX_INCONSISTENT");
		return {
			resource_id: document.resource_id,
			revision: document.revision,
			kind: document.target.kind,
			app_id: document.target.app_id,
			board_id: document.target.board_id,
			title: boundedWorkspaceText(document.title, 512),
			symbol: document.symbol,
			signature: document.signature
				? boundedWorkspaceText(document.signature, 1_024)
				: undefined,
			source_path: document.source_path,
			start_line: document.start_line,
			end_line: document.end_line,
			anchor_id: document.anchor_id,
			snippet: snippet(document.content, Object.keys(result.match)),
			matched_fields: [...new Set(Object.values(result.match).flat())].sort(),
			score: Math.round(result.score * 1_000) / 1_000,
		};
	});
	return { hits, mode };
}

export function createWorkspaceIndex(documents: WorkspaceDocument[]) {
	return {
		search: (query: string) => rankWorkspaceDocuments(documents, query),
	};
}
