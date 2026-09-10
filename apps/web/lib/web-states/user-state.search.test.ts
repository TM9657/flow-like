import { afterEach, describe, expect, it, vi } from "vitest";
import type { WebBackendRef } from "./api-utils";
import { WebUserState } from "./user-state";

const state = () =>
	new WebUserState({
		auth: { user: { access_token: "test-token" } },
	} as WebBackendRef);

afterEach(() => vi.unstubAllGlobals());

describe("project user search HTTP adapter", () => {
	it("encodes the identity and target project independently", async () => {
		const request = vi.fn(async () =>
			Response.json([{ id: "person", exact_match: true }]),
		);
		vi.stubGlobal("fetch", request);
		expect(
			await state().searchUsers("  person+tag@example.com  ", "project/?&"),
		).toEqual([{ id: "person", exact_match: true }]);
		const [url, options] = request.mock.calls[0] as unknown as [
			string,
			RequestInit,
		];
		const parsed = new URL(url);
		expect(decodeURIComponent(parsed.pathname)).toBe(
			"/api/v1/user/search/person+tag@example.com",
		);
		expect(parsed.searchParams.get("app_id")).toBe("project/?&");
		expect(parsed.searchParams.get("limit")).toBe("25");
		expect(new Headers(options.headers).get("Authorization")).toBe(
			"Bearer test-token",
		);
	});

	it("preserves the contacts cursor and bounded page size", async () => {
		const page = {
			users: [{ id: "person", name: null }],
			next_cursor: "person",
		};
		const request = vi.fn(async () => Response.json(page));
		vi.stubGlobal("fetch", request);
		expect(
			await state().getProjectContacts("target", "provider|last+person"),
		).toEqual(page);
		const [url] = request.mock.calls[0] as unknown as [string];
		const parsed = new URL(url);
		expect(parsed.pathname).toBe("/api/v1/user/contacts");
		expect(Object.fromEntries(parsed.searchParams)).toEqual({
			app_id: "target",
			after: "provider|last+person",
			limit: "500",
		});
	});

	it("reports failures from either source instead of an empty result", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn(async () =>
				Response.json({ error: "unavailable" }, { status: 503 }),
			),
		);
		await expect(state().searchUsers("Alice", "target")).rejects.toThrow();
		await expect(state().getProjectContacts("target")).rejects.toThrow();
	});
});
