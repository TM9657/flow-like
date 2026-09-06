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

describe("Home draft ownership", () => {
	it("does not let an older Save or effect clear or replace a resumed draft", () => {
		const cache = new HomeDraftStore();
		const first = cache.claim("profile-home");
		cache.set(first, draft("Old save"));
		const second = cache.claim("profile-home");
		cache.set(second, draft("New unsaved changes"));
		expect(cache.discard(first)).toBe(false);
		cache.set(first, draft("Late old effect"));
		expect(cache.get("profile-home")?.layout.title).toBe("New unsaved changes");
		expect(cache.discard(second)).toBe(true);
		expect(cache.get("profile-home")).toBeUndefined();
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
