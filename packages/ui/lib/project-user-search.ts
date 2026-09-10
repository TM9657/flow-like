import type { IUserLookup } from "../state/backend-state/types";

const DEFAULT_LIMIT = 25;
const PROJECT_BOOST = 80;

interface SearchEntry {
	user: IUserLookup;
	id: string;
	email: string;
	handles: string[];
	name: string;
	tokens: string[];
}

interface SearchQuery {
	text: string;
	tokens: string[];
	handleOnly: boolean;
	emailOnly: boolean;
}

interface RankedUser {
	user: IUserLookup;
	fromProject: boolean;
	score: number;
}

function normalize(value?: string | null): string {
	return (value ?? "")
		.normalize("NFKD")
		.replace(/\p{M}/gu, "")
		.toLowerCase()
		.replace(/ß/g, "ss")
		.replace(/ς/g, "σ")
		.replace(/\s+/g, " ")
		.trim();
}

function words(value: string): string[] {
	return value.match(/[\p{L}\p{N}]+/gu) ?? [];
}

function createEntry(user: IUserLookup): SearchEntry {
	const name = normalize(user.name);
	const email = normalize(user.email);
	const handles = Array.from(
		new Set(
			[user.preferred_username, user.username]
				.map((value) => normalize(value).replace(/^@/, ""))
				.filter(Boolean),
		),
	);
	return {
		user,
		id: normalize(user.id),
		email,
		handles,
		name,
		tokens: Array.from(new Set(words([name, email, ...handles].join(" ")))),
	};
}

function createQuery(value: string): SearchQuery {
	const normalized = normalize(value);
	const handleOnly = normalized.startsWith("@");
	const text = handleOnly ? normalized.slice(1) : normalized;
	return {
		text,
		tokens: words(text),
		handleOnly,
		emailOnly: !handleOnly && text.includes("@"),
	};
}

// One insertion, deletion, replacement, or adjacent transposition. No edit matrix
// is allocated while scanning contacts on each keystroke.
function oneTypo(left: string, right: string): boolean {
	if (left.length < 4 || Math.abs(left.length - right.length) > 1) {
		return false;
	}
	let first = 0;
	while (first < left.length && left[first] === right[first]) first++;
	if (first === left.length) return true;
	let leftIndex = first;
	let rightIndex = first;
	if (left.length === right.length) {
		if (left[first] === right[first + 1] && left[first + 1] === right[first]) {
			leftIndex += 2;
			rightIndex += 2;
		} else {
			leftIndex++;
			rightIndex++;
		}
	} else if (left.length > right.length) {
		leftIndex++;
	} else {
		rightIndex++;
	}
	while (leftIndex < left.length && rightIndex < right.length) {
		if (left[leftIndex++] !== right[rightIndex++]) return false;
	}
	return true;
}

function scoreEntry(entry: SearchEntry, query: SearchQuery): number {
	const { text, tokens } = query;
	if (!text) return query.handleOnly ? 0 : 1;
	if (query.handleOnly) {
		let score = 0;
		for (const handle of entry.handles) {
			if (handle === text) return 1_000;
			if (handle.startsWith(text)) score = 610;
			else if (oneTypo(text, handle)) score = Math.max(score, 280);
		}
		return score;
	}
	if (
		entry.id === text ||
		entry.email === text ||
		entry.handles.includes(text)
	) {
		return 1_000;
	}
	if (query.emailOnly) return entry.email.startsWith(text) ? 610 : 0;
	if (entry.name === text) return 850;
	if (!tokens.length) return 0;

	let weakest = 740;
	let total = 0;
	for (const token of tokens) {
		let best = 0;
		for (const candidate of entry.tokens) {
			if (candidate === token) {
				best = 740;
				break;
			}
			if (candidate.startsWith(token)) best = Math.max(best, 610);
			else if (token.length >= 3 && candidate.includes(token)) {
				best = Math.max(best, 440);
			} else if (best < 280 && oneTypo(token, candidate)) {
				best = 280;
			}
		}
		if (!best) return 0;
		weakest = Math.min(weakest, best);
		total += best;
	}
	return weakest + total / tokens.length / 100;
}

function resultLimit(limit: number): number {
	return Number.isFinite(limit)
		? Math.max(0, Math.floor(limit))
		: DEFAULT_LIMIT;
}

function addResult(
	results: RankedUser[],
	user: IUserLookup,
	score: number,
	fromProject: boolean,
	limit: number,
): void {
	if (results.length === limit && score <= results[limit - 1].score) return;
	let position = results.length;
	while (position > 0 && results[position - 1].score < score) position--;
	results.splice(position, 0, { user, score, fromProject });
	if (results.length > limit) results.pop();
}

/** Build once when contacts change; each query retains only the best results. */
export function createProjectUserSearch(users: readonly IUserLookup[]): {
	search(query: string, limit?: number): IUserLookup[];
} {
	const byId = new Map<string, SearchEntry>();
	for (const user of users) byId.set(user.id, createEntry(user));
	const entries = Array.from(byId.values());
	return {
		search(query, limit = DEFAULT_LIMIT) {
			const cap = resultLimit(limit);
			if (!cap) return [];
			const parsed = createQuery(query);
			if (!parsed.text && !parsed.handleOnly) {
				return entries.slice(0, cap).map((entry) => entry.user);
			}
			const results: RankedUser[] = [];
			for (const entry of entries) {
				const score = scoreEntry(entry, parsed);
				if (score) addResult(results, entry.user, score, true, cap);
			}
			return results.map((result) => result.user);
		},
	};
}

/** Merge bounded local search results with directory results for the same query. */
export function mergeProjectUserResults(
	localUsers: readonly IUserLookup[],
	remoteUsers: readonly IUserLookup[],
	query: string,
	limit = DEFAULT_LIMIT,
): { user: IUserLookup; fromProject: boolean }[] {
	const cap = resultLimit(limit);
	if (!cap) return [];
	const parsed = createQuery(query);
	const localById = new Map(localUsers.map((user) => [user.id, user]));
	const remoteById = new Map(remoteUsers.map((user) => [user.id, user]));
	const results: RankedUser[] = [];
	for (const [id, local] of localById) {
		const remote = remoteById.get(id);
		const score = Math.max(
			scoreEntry(createEntry(local), parsed),
			remote ? scoreEntry(createEntry(remote), parsed) : 0,
			remote?.exact_match ? 1_000 : 0,
		);
		if (score || remote) {
			addResult(results, remote ?? local, score + PROJECT_BOOST, true, cap);
		}
	}
	for (const [id, user] of remoteById) {
		if (localById.has(id)) continue;
		// Keep server matches even when the directory has additional searchable fields.
		addResult(
			results,
			user,
			Math.max(
				scoreEntry(createEntry(user), parsed),
				user.exact_match ? 1_000 : 0,
			),
			false,
			cap,
		);
	}
	return results.map(({ user, fromProject }) => ({ user, fromProject }));
}
