import type { AuthContextProps } from "react-oidc-context";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { reportUnauthorized } from "./auth-session";
import { fetchWithSession } from "./session-request";

vi.mock("./auth-session", () => ({ reportUnauthorized: vi.fn() }));
type User = NonNullable<AuthContextProps["user"]>;
function user(token: string, expired = false, sub = "user-1"): User {
	return {
		access_token: token,
		expired,
		profile: { sub },
		refresh_token: "refresh",
	} as User;
}
function context(initial = user("old")) {
	const loaded = new Set<(user: User) => void>();
	const unloaded = new Set<() => void>();
	const auth = {
		isAuthenticated: true,
		user: initial,
		signinSilent: vi.fn(async () => user("new")),
		events: {
			addUserLoaded: (fn: (user: User) => void) => loaded.add(fn),
			addUserUnloaded: (fn: () => void) => unloaded.add(fn),
		},
	} as unknown as AuthContextProps;
	return {
		auth,
		load: (user: User) => {
			for (const fn of loaded) fn(user);
		},
		unload: () => {
			for (const fn of unloaded) fn();
		},
	};
}
const url = "https://hub.test/api/v1/user/info";
const request = (auth: AuthContextProps) =>
	fetchWithSession(
		url,
		{
			method: "POST",
			body: JSON.stringify({ value: 1 }),
			headers: { Authorization: `Bearer ${auth.user?.access_token}` },
		},
		auth,
	);

beforeEach(() => vi.mocked(reportUnauthorized).mockClear());
afterEach(() => vi.unstubAllGlobals());

describe("web session requests", () => {
	test("refreshes an expired token before sending the request", async () => {
		const { auth } = context(user("old", true));
		const fetcher = vi.fn(async (_url: string, init?: RequestInit) => {
			expect(new Headers(init?.headers).get("Authorization")).toBe(
				"Bearer new",
			);
			return new Response("ok");
		});
		vi.stubGlobal("fetch", fetcher);
		await request(auth);
		expect(fetcher).toHaveBeenCalledTimes(1);
		expect(auth.signinSilent).toHaveBeenCalledTimes(1);
	});

	test("concurrent 401s share refresh and reuse its token before React updates", async () => {
		const { auth } = context();
		const tokens: string[] = [];
		vi.stubGlobal("fetch", async (_url: string, init?: RequestInit) => {
			const token = new Headers(init?.headers).get("Authorization") ?? "";
			tokens.push(token);
			return new Response("", { status: token === "Bearer old" ? 401 : 200 });
		});
		const responses = await Promise.all([request(auth), request(auth)]);
		expect(responses.map((r) => r.status)).toEqual([200, 200]);
		expect(auth.signinSilent).toHaveBeenCalledTimes(1);
		expect((await request(auth)).status).toBe(200);
		expect(tokens).toEqual([
			"Bearer old",
			"Bearer old",
			"Bearer new",
			"Bearer new",
			"Bearer new",
		]);
		expect(reportUnauthorized).not.toHaveBeenCalled();
	});

	test("reports the replacement token if the single retry is also rejected", async () => {
		const { auth } = context();
		const fetcher = vi.fn(async () => new Response("", { status: 401 }));
		vi.stubGlobal("fetch", fetcher);
		expect((await request(auth)).status).toBe(401);
		expect(fetcher).toHaveBeenCalledTimes(2);
		expect(auth.signinSilent).toHaveBeenCalledTimes(1);
		expect(reportUnauthorized).toHaveBeenCalledExactlyOnceWith(auth, "new");
	});

	test("a temporary renewal failure leaves the session available for another attempt", async () => {
		const { auth } = context(user("old", true));
		vi.mocked(auth.signinSilent).mockRejectedValueOnce(
			new TypeError("Offline"),
		);
		vi.stubGlobal(
			"fetch",
			vi.fn(async () => new Response("ok")),
		);
		await expect(request(auth)).rejects.toThrow("Offline");
		expect(reportUnauthorized).not.toHaveBeenCalled();
		expect((await request(auth)).status).toBe(200);
		expect(auth.signinSilent).toHaveBeenCalledTimes(2);
	});

	test("does not retry a request under another account or a signed-out session", async () => {
		for (const change of ["signout", "switch"]) {
			const state = context();
			const fetcher = vi.fn(async () => {
				if (change === "signout") state.unload();
				else state.load(user("other", false, "user-2"));
				return new Response("", { status: 401 });
			});
			vi.stubGlobal("fetch", fetcher);
			await request(state.auth);
			expect(fetcher).toHaveBeenCalledTimes(1);
			expect(state.auth.signinSilent).not.toHaveBeenCalled();
		}
	});

	test("keeps a caller's separate bearer token outside account recovery", async () => {
		const { auth } = context(user("old", true));
		vi.stubGlobal("fetch", async (_url: string, init?: RequestInit) => {
			expect(new Headers(init?.headers).get("Authorization")).toBe(
				"Bearer separate",
			);
			return new Response("", { status: 401 });
		});
		await fetchWithSession(
			url,
			{ headers: { Authorization: "Bearer separate" } },
			auth,
		);
		expect(auth.signinSilent).not.toHaveBeenCalled();
		expect(reportUnauthorized).not.toHaveBeenCalled();
	});
});
