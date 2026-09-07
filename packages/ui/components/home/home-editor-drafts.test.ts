import { describe, expect, it } from "bun:test";
import { HomeDraftStore, type HomeEditorDraft } from "./home-editor-drafts";
import type { IHomeLayout } from "./types";

const layout = (title: string): IHomeLayout => ({
	version: 1,
	title,
	widgets: [],
});
const draft = (
	title: string,
	baseRevision: string | null = "r1",
): HomeEditorDraft => ({
	layout: layout(title),
	reset: false,
	baseLayout: layout("Published"),
	baseRevision,
});

const memoryStorage = () => {
	const entries = new Map<string, string>();
	return {
		entries,
		getItem: (key: string) => entries.get(key) ?? null,
		setItem: (key: string, value: string) => {
			entries.set(key, value);
		},
		removeItem: (key: string) => {
			entries.delete(key);
		},
	};
};

describe("Home draft ownership", () => {
	it("does not let an older Save or effect clear or replace a resumed draft", () => {
		const storage = memoryStorage();
		const cache = new HomeDraftStore(() => storage);
		const first = cache.claim("profile-home");
		cache.set(first, draft("Old save"));
		const second = cache.claim("profile-home");
		cache.set(second, draft("New unsaved changes"));
		expect(cache.discard(first)).toBe(false);
		cache.set(first, draft("Late old effect"));
		expect(cache.get("profile-home")?.layout.title).toBe("New unsaved changes");
		expect(
			new HomeDraftStore(() => storage).get("profile-home")?.layout.title,
		).toBe("New unsaved changes");
		expect(cache.discard(second)).toBe(true);
		expect(cache.get("profile-home")).toBeUndefined();
		expect(
			new HomeDraftStore(() => storage).get("profile-home"),
		).toBeUndefined();
	});

	it("transfers ownership as soon as editing resumes, before a new change", () => {
		const cache = new HomeDraftStore();
		const first = cache.claim("main");
		cache.set(first, draft("Pending"));
		const second = cache.claim("main");
		expect(cache.discard(first)).toBe(false);
		expect(cache.get("main")?.layout.title).toBe("Pending");
		cache.discard(second);
		expect(cache.get("main")).toBeUndefined();
	});

	it.each([null, "r1"])(
		"preserves the draft's original revision %s across recovery",
		(revision) => {
			const cache = new HomeDraftStore();
			cache.set(cache.claim("default"), draft("Recovered", revision));
			cache.claim("default");
			expect(cache.get("default")).toEqual(draft("Recovered", revision));
		},
	);

	it("keeps target scopes separate and only releases empty sessions", () => {
		const cache = new HomeDraftStore();
		const main = cache.claim("main");
		const template = cache.claim("template");
		cache.set(main, draft("Main"));
		cache.releaseEmpty(main);
		cache.releaseEmpty(template);
		cache.set(template, draft("Unmounted"));
		expect(cache.get("main")?.layout.title).toBe("Main");
		expect(cache.get("template")).toBeUndefined();
	});
});

describe("Home draft reload recovery", () => {
	it.each([undefined, null, "r1"])(
		"recovers changes, reset intent and original revision %s after a reload",
		(baseRevision) => {
			const storage = memoryStorage();
			const beforeReload = new HomeDraftStore(() => storage);
			const pending = {
				...draft("Recovered"),
				reset: true,
				baseRevision,
			};
			beforeReload.set(beforeReload.claim("profile-home"), pending);
			const afterReload = new HomeDraftStore(() => storage);
			expect(afterReload.get("profile-home")).toEqual(pending);
			const session = afterReload.claim("profile-home");
			afterReload.releaseEmpty(session);
			expect(afterReload.get("profile-home")).toEqual(pending);
			afterReload.discard(session);
			expect(
				new HomeDraftStore(() => storage).get("profile-home"),
			).toBeUndefined();
		},
	);

	it("keeps recovery scoped to the origin, viewer and profile supplied by the editor", () => {
		const storage = memoryStorage();
		const beforeReload = new HomeDraftStore(() => storage);
		const scopes = [
			"origin-a:viewer-a:profile-a",
			"origin-b:viewer-a:profile-a",
			"origin-a:viewer-b:profile-a",
			"origin-a:viewer-a:profile-b",
		];
		for (const scope of scopes)
			beforeReload.set(beforeReload.claim(scope), draft(scope));
		beforeReload.set(beforeReload.claim(undefined), draft("Unscoped"));
		expect(storage.entries.size).toBe(scopes.length);
		const afterReload = new HomeDraftStore(() => storage);
		for (const scope of scopes)
			expect(afterReload.get(scope)?.layout.title).toBe(scope);
		expect(afterReload.get(undefined)).toBeUndefined();
	});

	it("recovers editable content that still needs shortening before saving", () => {
		const storage = memoryStorage();
		const beforeReload = new HomeDraftStore(() => storage);
		const pending = draft("x".repeat(300));
		beforeReload.set(beforeReload.claim("profile-home"), pending);
		expect(new HomeDraftStore(() => storage).get("profile-home")).toEqual(
			pending,
		);
	});

	it.each([
		"{",
		"null",
		"[]",
		JSON.stringify({ ...draft("Invalid"), reset: "false" }),
		JSON.stringify({ ...draft("Invalid"), baseRevision: {} }),
		JSON.stringify({ ...draft("Invalid"), baseLayout: { version: 2 } }),
		JSON.stringify({
			...draft("Invalid"),
			layout: {
				version: 1,
				widgets: [
					{ id: "duplicate", type: "information" },
					{ id: "duplicate", type: "information" },
				],
			},
		}),
	])("ignores and removes corrupt recovery data: %s", (source) => {
		const storage = memoryStorage();
		storage.setItem("flow-like:home-draft:v1:profile-home", source);
		const cache = new HomeDraftStore(() => storage);
		expect(cache.get("profile-home")).toBeUndefined();
		expect(storage.entries.size).toBe(0);
		cache.set(cache.claim("profile-home"), draft("Replacement"));
		expect(new HomeDraftStore(() => storage).get("profile-home")).toEqual(
			draft("Replacement"),
		);
	});

	it("continues editing when browser storage access is blocked", () => {
		const cache = new HomeDraftStore(() => {
			throw new Error("Storage access denied");
		});
		expect(cache.get("profile-home")).toBeUndefined();
		const session = cache.claim("profile-home");
		cache.set(session, draft("In memory"));
		expect(cache.get("profile-home")).toEqual(draft("In memory"));
		expect(cache.discard(session)).toBe(true);
		expect(cache.get("profile-home")).toBeUndefined();
	});

	it("keeps the latest draft in memory when storage quota is exhausted", () => {
		const storage = memoryStorage();
		storage.setItem = () => {
			throw new Error("Quota exceeded");
		};
		const cache = new HomeDraftStore(() => storage);
		const session = cache.claim("profile-home");
		cache.set(session, draft("In memory"));
		expect(cache.get("profile-home")).toEqual(draft("In memory"));
		expect(cache.discard(session)).toBe(true);
	});

	it("does not resurrect a discarded draft in memory if storage removal fails", () => {
		const storage = memoryStorage();
		const cache = new HomeDraftStore(() => storage);
		const first = cache.claim("profile-home");
		cache.set(first, draft("Discarded"));
		storage.removeItem = () => {
			throw new Error("Storage unavailable");
		};
		expect(cache.discard(first)).toBe(true);
		expect(cache.get("profile-home")).toBeUndefined();
		const second = cache.claim("profile-home");
		cache.set(first, draft("Late old effect"));
		expect(cache.get("profile-home")).toBeUndefined();
		cache.set(second, draft("New changes"));
		expect(cache.get("profile-home")).toEqual(draft("New changes"));
	});
});
