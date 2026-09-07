import type { IHomeLayout } from "@flow-like/flow-like-ui/components/home/types";
import { afterEach, describe, expect, test, vi } from "vitest";
import type { WebBackendRef } from "./api-utils";
import { WebUserState } from "./user-state";

const layout: IHomeLayout = {
	version: 1,
	title: "My workspace",
	widgets: [
		{
			id: "note",
			type: "markdown",
			size: { columns: 6, rows: 2 },
			appearance: { variant: "card", accent: "blue" },
			config: { markdown: "Keep this note", nested: { a: 1, b: 2 } },
		},
	],
};

function profile(homeLayout: IHomeLayout | null = null) {
	return {
		id: "profile-1",
		name: "Workspace",
		hub: "api.example.test",
		user_id: "user-1",
		created_at: "2026-09-07T10:00:00Z",
		updated_at: "2026-09-07T10:01:00Z",
		home_layout: homeLayout,
		home_default_id: "personal",
	};
}

function state() {
	return new WebUserState({
		auth: { user: { access_token: "test-token" } },
	} as WebBackendRef);
}

function respond(body: unknown, status = 200) {
	const request = vi.fn(async () => Response.json(body, { status }));
	vi.stubGlobal("fetch", request);
	return request;
}

function reorderKeys(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(reorderKeys);
	if (value && typeof value === "object") {
		return Object.fromEntries(
			Object.entries(value)
				.reverse()
				.map(([key, entry]) => [key, reorderKeys(entry)]),
		);
	}
	return value;
}

afterEach(() => {
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

describe("WebUserState home saves through the HTTP adapter", () => {
	test("rejects an HTTP 200 from a hub that silently ignores the home field", async () => {
		const { home_layout: _layout, ...legacyProfile } = profile();
		respond({ profile: legacyProfile });

		await expect(state().saveHomeLayout(layout, "profile-1")).rejects.toThrow(
			"This server does not support saving home layouts yet. Your changes are kept as a draft.",
		);
	});

	test("does not mistake an omitted field for an acknowledged reset", async () => {
		const { home_layout: _layout, ...legacyProfile } = profile();
		respond({ profile: legacyProfile });

		await expect(state().saveHomeLayout(null, "profile-1")).rejects.toThrow(
			"does not support saving home layouts",
		);
	});

	test.each([
		["a missing custom layout", profile(null)],
		["a different custom layout", profile({ ...layout, title: "Old layout" })],
		["a different profile", { ...profile(layout), id: "profile-2" }],
	])("rejects a successful response containing %s", async (_name, stored) => {
		respond({ profile: stored });
		await expect(state().saveHomeLayout(layout, "profile-1")).rejects.toThrow(
			"The server did not confirm your home layout",
		);
	});

	test("accepts the stored layout after JSONB reorders nested object keys", async () => {
		const request = respond({ profile: reorderKeys(profile(layout)) });
		await expect(
			state().saveHomeLayout(layout, "profile-1"),
		).resolves.toMatchObject({
			id: "profile-1",
			home_layout: layout,
			home_default_id: "personal",
			updated: profile().updated_at,
		});
		const [url, options] = request.mock.calls[0] as unknown as [
			string,
			RequestInit,
		];
		expect(url).toContain("/api/v1/profile/profile-1");
		expect(options.method).toBe("POST");
		expect(JSON.parse(options.body as string)).toEqual({ home_layout: layout });
		expect(new Headers(options.headers).get("Authorization")).toBe(
			"Bearer test-token",
		);
	});

	test("accepts an explicit stored null when resetting to the inherited layout", async () => {
		respond({ profile: profile(null) });
		await expect(
			state().saveHomeLayout(null, "profile-1"),
		).resolves.toMatchObject({
			id: "profile-1",
			home_layout: null,
			updated: profile().updated_at,
		});
	});

	test("propagates a rejected write instead of acknowledging it", async () => {
		vi.spyOn(console, "error").mockImplementation(() => {});
		respond({ message: "Home layout exceeds the limit" }, 400);
		await expect(state().saveHomeLayout(layout, "profile-1")).rejects.toThrow();
	});

	test("reads the saved layout and its default from a fresh backend instance", async () => {
		let stored = profile();
		vi.stubGlobal(
			"fetch",
			vi.fn(async (_url: string, options: RequestInit) => {
				if (options.method === "POST") {
					const body = JSON.parse(options.body as string);
					stored = { ...stored, home_layout: body.home_layout };
					return Response.json({ profile: stored });
				}
				return Response.json([stored]);
			}),
		);

		await state().saveHomeLayout(layout, "profile-1");
		const reloaded = await state().getProfile();
		expect(reloaded.home_layout).toEqual(layout);
		expect(reloaded.home_default_id).toBe("personal");
		expect(reloaded.updated).toBe(stored.updated_at);
	});
});
