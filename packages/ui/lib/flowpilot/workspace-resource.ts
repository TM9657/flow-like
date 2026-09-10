export const WORKSPACE_KINDS = [
	"workflow",
	"event",
	"table",
	"page",
	"doc",
] as const;
export type WorkspaceKind = (typeof WORKSPACE_KINDS)[number];

export interface WorkspaceTarget {
	kind: WorkspaceKind;
	app_id?: string;
	board_id?: string;
	id: string;
}

export interface WorkspaceDocument {
	resource_id: string;
	target: WorkspaceTarget;
	revision: string;
	title: string;
	symbol?: string;
	signature?: string;
	content: string;
	source_path?: string;
	start_line?: number;
	end_line?: number;
	anchor_id?: string;
	/** Contract fields or docs omitted by a bounded projection. */
	truncated?: boolean;
}

export interface WorkspaceCoverage {
	complete: boolean;
	indexed_resources: number;
	app_ids: string[];
	kinds: WorkspaceKind[];
	issues: Array<{
		kind?: WorkspaceKind;
		app_id?: string;
		resource?: string;
		code: string;
	}>;
	docs?: unknown;
}

const encoder = new TextEncoder();
export const jsonBytes = (value: unknown) =>
	encoder.encode(JSON.stringify(value)).length;

export function stableJson(value: unknown): string {
	const normalize = (item: unknown): unknown => {
		if (Array.isArray(item)) return item.map(normalize);
		if (!item || typeof item !== "object") return item;
		return Object.fromEntries(
			Object.entries(item)
				.sort(([a], [b]) => a.localeCompare(b))
				.map(([key, value]) => [key, normalize(value)]),
		);
	};
	return JSON.stringify(normalize(value), null, 2);
}

export async function workspaceRevision(content: string): Promise<string> {
	const hash = await crypto.subtle.digest("SHA-256", encoder.encode(content));
	return Array.from(new Uint8Array(hash), (byte) =>
		byte.toString(16).padStart(2, "0"),
	).join("");
}

export function workspaceResourceId(target: WorkspaceTarget): string {
	return `workspace:v1:${encodeURIComponent(JSON.stringify([target.kind, target.app_id ?? null, target.board_id ?? null, target.id]))}`;
}

export function parseWorkspaceResourceId(
	value: unknown,
): WorkspaceTarget | undefined {
	if (
		typeof value !== "string" ||
		value.length > 4_096 ||
		!value.startsWith("workspace:v1:")
	)
		return undefined;
	try {
		const parts: unknown = JSON.parse(decodeURIComponent(value.slice(13)));
		if (!Array.isArray(parts) || parts.length !== 4) return undefined;
		const [kind, app, board, id] = parts;
		if (
			!WORKSPACE_KINDS.includes(kind) ||
			typeof id !== "string" ||
			!id ||
			id.length > 512
		)
			return undefined;
		if (app !== null && (typeof app !== "string" || !app || app.length > 256))
			return undefined;
		if (
			board !== null &&
			(typeof board !== "string" || !board || board.length > 256)
		)
			return undefined;
		if (kind === "doc" ? app !== null || board !== null : app === null)
			return undefined;
		if (kind === "workflow" && board === null) return undefined;
		const target: WorkspaceTarget = {
			kind,
			id,
			...(app === null ? {} : { app_id: app }),
			...(board === null ? {} : { board_id: board }),
		};
		return workspaceResourceId(target) === value ? target : undefined;
	} catch {
		return undefined;
	}
}

/** Bound serialized text, including escapes, without splitting a UTF-16 surrogate pair. */
export function boundedWorkspaceText(value: string, budget: number): string {
	if (jsonBytes(value) <= budget) return value;
	let low = 0;
	let high = Math.min(value.length, budget);
	while (low < high) {
		const mid = Math.ceil((low + high) / 2);
		if (jsonBytes(value.slice(0, mid)) <= budget) low = mid;
		else high = mid - 1;
	}
	if (low > 0 && /[\uD800-\uDBFF]/.test(value[low - 1])) low--;
	return value.slice(0, low);
}
