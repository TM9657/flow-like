import MiniSearch, { type SearchResult } from "minisearch";
import { tokenizeSearchText } from "../search-index";
import {
	type WorkspaceDocument,
	type WorkspaceKind,
	boundedWorkspaceText,
} from "./workspace-resource";

const STOP_WORDS = new Set(
	"a an the and or for with to of in on at by from as is are was were be been being do does did can could should would will may might how what which where when who why that this these those it its then than while before after please find look looking show get give existing example examples helper helpers function functions workflow workflows flow flows code implementation implements implement use using used keep appropriate new also about into over under".split(
		" ",
	),
);

function normalizeTerm(raw: string): string | undefined {
	let term = raw.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase();
	if (STOP_WORDS.has(term)) return undefined;
	if (/^[a-z]{5,}$/.test(term)) {
		if (term.endsWith("ies")) term = `${term.slice(0, -3)}y`;
		else if (/(?:sses|shes|ches|xes|zes)$/.test(term)) term = term.slice(0, -2);
		else if (term.endsWith("s") && !/(?:ss|us|is)$/.test(term))
			term = term.slice(0, -1);
		if (term.length > 6 && term.endsWith("ing"))
			term = term.slice(0, -3).replace(/([bdgmnprt])\1$/, "$1");
		else if (term.length > 5 && term.endsWith("ed"))
			term = term.slice(0, -2).replace(/([bdgmnprt])\1$/, "$1");
		else if (term.length > 6 && term.endsWith("ly")) term = term.slice(0, -2);
		if (term.length > 4 && term.endsWith("e")) term = term.slice(0, -1);
	}
	return term || undefined;
}

function termsFor(text: string): string[] {
	return [
		...new Set(
			tokenizeSearchText(text)
				.map(normalizeTerm)
				.filter((term): term is string => Boolean(term)),
		),
	];
}

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
	match_quality?: "identifier" | "strict" | "partial";
	query_coverage?: number;
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

/** Build once per observed corpus. Every exact read still resolves the current source. */
export function createWorkspaceIndex(documents: WorkspaceDocument[]) {
	const unique = [
		...new Map(
			documents.map((document) => [document.resource_id, document]),
		).values(),
	];
	const byId = new Map(
		unique.map((document) => [document.resource_id, document]),
	);
	const frequency = new Map<string, number>();
	const exactNames = new Map<string, Set<string>>();
	const index = new MiniSearch({
		idField: "resource_id",
		fields: ["title", "symbol", "signature", "content"],
		tokenize: tokenizeSearchText,
		processTerm: normalizeTerm,
		searchOptions: {
			combineWith: "AND",
			prefix: true,
			boost: { symbol: 4, title: 3, signature: 2 },
			maxFuzzy: 2,
		},
	});
	index.addAll(unique);
	for (const document of unique) {
		for (const term of termsFor(
			[document.title, document.symbol, document.signature, document.content]
				.filter(Boolean)
				.join(" "),
		))
			frequency.set(term, (frequency.get(term) ?? 0) + 1);
		for (const name of [
			document.symbol,
			document.symbol?.split("::").at(-1),
			document.title,
		]) {
			if (!name) continue;
			const key = name.trim().toLowerCase();
			const ids = exactNames.get(key) ?? new Set<string>();
			ids.add(document.resource_id);
			exactNames.set(key, ids);
		}
	}
	return {
		search(query: string): { hits: WorkspaceHit[]; mode: string } {
			const exactIds =
				exactNames.get(query.trim().toLowerCase()) ?? new Set<string>();
			let terms = termsFor(query);
			if (!terms.length && exactIds.size) terms = [query.trim().toLowerCase()];
			if (!terms.length) return { hits: [], mode: "no_meaningful_terms" };
			const requiredAcronyms = tokenizeSearchText(query)
				.filter((term) => /^[A-Z][A-Z0-9]{1,}$/.test(term))
				.map(normalizeTerm)
				.filter((term): term is string => Boolean(term));
			// Pass normalized tokens through once; stemming a second time changes words such as "classes".
			const options = {
				tokenize: (text: string) => text.split(" "),
				processTerm: (term: string) => term,
			};
			const queryText = terms.join(" ");
			let strict = index.search(queryText, options);
			let mode = "prefix";
			if (!strict.length) {
				strict = index.search(queryText, {
					...options,
					fuzzy: (term: string) => (term.length >= 5 ? 0.2 : false),
				});
				mode = "fuzzy";
			}
			const identifierQuery =
				!/\s/.test(query.trim()) && /[a-z][A-Z]|_|::|\d/.test(query);
			const strictIds = new Set(strict.map((result) => String(result.id)));
			const candidates = new Map<string, SearchResult>(
				strict.map((result) => [String(result.id), result]),
			);
			for (const id of exactIds) {
				if (candidates.has(id)) continue;
				candidates.set(id, {
					id,
					score: Math.max(1, ...strict.map((item) => item.score)),
					terms,
					queryTerms: terms,
					match: Object.fromEntries(
						terms.map((term) => [
							term,
							byId.get(id)?.symbol ? ["title", "symbol"] : ["title"],
						]),
					),
				});
				strictIds.add(id);
			}
			// Keep multiword candidates even when a strict match exists; an orchestrator and its helper
			// often describe the same operation with different amounts of surrounding text.
			if (!identifierQuery && terms.length >= 2) {
				for (const result of index.search(queryText, {
					...options,
					combineWith: "OR",
					prefix: true,
				}))
					if (!candidates.has(String(result.id)))
						candidates.set(String(result.id), result);
			}
			const weights = new Map(
				terms.map((term) => [
					term,
					Math.min(
						4,
						1 + Math.log(1 + unique.length / (1 + (frequency.get(term) ?? 0))),
					),
				]),
			);
			const totalWeight = [...weights.values()].reduce(
				(sum, weight) => sum + weight,
				0,
			);
			const ranked = [];
			for (const result of candidates.values()) {
				const id = String(result.id);
				const matched = new Set(result.queryTerms);
				const coverage =
					terms.reduce(
						(sum, term) =>
							sum + (matched.has(term) ? (weights.get(term) ?? 0) : 0),
						0,
					) / totalWeight;
				const strictMatch = strictIds.has(id);
				if (
					!exactIds.has(id) &&
					requiredAcronyms.some((term) => !matched.has(term))
				)
					continue;
				if (!strictMatch && (matched.size < 2 || coverage < 0.5)) continue;
				const exact = exactIds.has(id);
				const score =
					result.score *
					coverage *
					coverage *
					(exact ? 8 : strictMatch ? 1.25 : 1);
				ranked.push({
					result,
					score,
					coverage,
					quality: exact
						? ("identifier" as const)
						: strictMatch
							? ("strict" as const)
							: ("partial" as const),
				});
			}
			ranked.sort(
				(a, b) =>
					b.score - a.score ||
					String(a.result.id).localeCompare(String(b.result.id)),
			);
			if (ranked.some((item) => item.quality === "partial"))
				mode = strict.length ? "mixed" : "partial";
			const hits = ranked.map(
				({ result, score, coverage, quality }): WorkspaceHit => {
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
							? boundedWorkspaceText(document.signature, 1024)
							: undefined,
						source_path: document.source_path,
						start_line: document.start_line,
						end_line: document.end_line,
						anchor_id: document.anchor_id,
						snippet: snippet(document.content, Object.keys(result.match)),
						matched_fields: [
							...new Set(Object.values(result.match).flat()),
						].sort(),
						score: Math.round(score * 1000) / 1000,
						match_quality: quality,
						query_coverage: Math.round(coverage * 1000) / 1000,
					};
				},
			);
			return { hits, mode };
		},
	};
}

/** One-shot entry point for callers that do not retain a corpus between queries. */
export function rankWorkspaceDocuments(
	documents: WorkspaceDocument[],
	query: string,
) {
	return createWorkspaceIndex(documents).search(query);
}
