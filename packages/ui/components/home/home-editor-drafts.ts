import { normalizeHomeLayout } from "./home-layout";
import {
	formatJsonDocument,
	parseJsonDocument,
} from "./home-layout-json-document";
import type { IHomeLayout } from "./types";

export interface HomeEditorDraft {
	layout: IHomeLayout;
	reset: boolean;
	baseLayout: IHomeLayout;
	baseRevision: string | null | undefined;
}

export interface HomeDraftSession {
	readonly key: string | undefined;
	readonly owner: symbol;
}

type HomeDraftStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

const storageKey = (key: string) => `flow-like:home-draft:v1:${key}`;

function sessionStorage(): HomeDraftStorage | undefined {
	return typeof window === "undefined" ? undefined : window.sessionStorage;
}

function recoverDraft(source: string): HomeEditorDraft | undefined {
	const parsed = parseJsonDocument(source);
	if (!parsed.ok) return;
	const value = parsed.value;
	if (
		!value ||
		typeof value !== "object" ||
		Array.isArray(value) ||
		!("reset" in value) ||
		typeof value.reset !== "boolean" ||
		!("layout" in value) ||
		!("baseLayout" in value)
	)
		return;
	const baseRevision = "baseRevision" in value ? value.baseRevision : undefined;
	if (
		baseRevision !== undefined &&
		baseRevision !== null &&
		typeof baseRevision !== "string"
	)
		return;
	const layout = normalizeHomeLayout(value.layout);
	const baseLayout = normalizeHomeLayout(value.baseLayout);
	if (!layout || !baseLayout) return;
	return {
		layout,
		baseLayout,
		reset: value.reset,
		baseRevision,
	};
}

/** Session ownership keeps a late save from clearing another editor's recovered draft. */
export class HomeDraftStore {
	private readonly entries = new Map<
		string,
		{ owner: symbol; draft?: HomeEditorDraft }
	>();

	constructor(
		private readonly getStorage: () =>
			| HomeDraftStorage
			| undefined = sessionStorage,
	) {}

	get(key: string | undefined) {
		if (!key) return;
		if (this.entries.has(key)) return this.entries.get(key)?.draft;
		try {
			const storage = this.getStorage();
			const source = storage?.getItem(storageKey(key));
			if (!source) return;
			const draft = recoverDraft(source);
			if (!draft) {
				storage?.removeItem(storageKey(key));
				return;
			}
			this.entries.set(key, { owner: Symbol("recovered-home-draft"), draft });
			return draft;
		} catch {
			// Browser storage can be disabled without preventing editing.
			return;
		}
	}

	claim(key: string | undefined): HomeDraftSession {
		const session = { key, owner: Symbol("home-edit-session") };
		if (key)
			this.entries.set(key, { owner: session.owner, draft: this.get(key) });
		return session;
	}

	set(session: HomeDraftSession | null, draft: HomeEditorDraft) {
		if (!session?.key) return;
		const entry = this.entries.get(session.key);
		if (entry?.owner !== session.owner) return;
		entry.draft = draft;
		try {
			const serialized = formatJsonDocument(draft);
			if (serialized.ok)
				this.getStorage()?.setItem(storageKey(session.key), serialized.json);
		} catch {
			// Keep the in-memory draft if storage is unavailable or full.
		}
	}

	discard(session: HomeDraftSession | null) {
		if (!session?.key) return false;
		if (this.entries.get(session.key)?.owner !== session.owner) return false;
		// Retain a tombstone so a failed removal cannot resurrect the draft in this tab.
		this.entries.set(session.key, { owner: Symbol("discarded-home-draft") });
		try {
			this.getStorage()?.removeItem(storageKey(session.key));
		} catch {
			// The draft is still discarded in memory when browser storage is unavailable.
		}
		return true;
	}

	releaseEmpty(session: HomeDraftSession | null) {
		if (session?.key && !this.get(session.key)) this.discard(session);
	}
}
