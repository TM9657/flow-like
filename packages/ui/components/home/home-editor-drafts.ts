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

/** Session ownership keeps a late save from clearing another editor's recovered draft. */
export class HomeDraftStore {
	private readonly entries = new Map<
		string,
		{ owner: symbol; draft?: HomeEditorDraft }
	>();

	get(key: string | undefined) {
		return key ? this.entries.get(key)?.draft : undefined;
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
		if (entry?.owner === session.owner) entry.draft = draft;
	}

	discard(session: HomeDraftSession | null) {
		if (!session?.key) return false;
		if (this.entries.get(session.key)?.owner !== session.owner) return false;
		return this.entries.delete(session.key);
	}

	releaseEmpty(session: HomeDraftSession | null) {
		if (session?.key && !this.get(session.key)) this.discard(session);
	}
}
