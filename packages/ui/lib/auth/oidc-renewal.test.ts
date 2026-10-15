import { describe, expect, test } from "bun:test";
import {
	coordinateOidcRenewal,
	renewOidcSession,
	requiresInteractiveSignIn,
} from "./oidc-renewal";

type User = {
	access_token: string;
	refresh_token: string;
	expired: boolean;
	profile: { sub: string };
};
const user = (token: string): User => ({
	access_token: token,
	refresh_token: `refresh-${token}`,
	expired: false,
	profile: { sub: "account" },
});
const deferred = <T>() => Promise.withResolvers<T>();

function manager(
	store: { user: User | null },
	renew: () => Promise<User | null>,
	authority = "https://issuer.example",
) {
	let loaded = 0;
	return {
		settings: { authority, client_id: "client" },
		events: {},
		get loaded() {
			return loaded;
		},
		async getUser(raiseEvent?: boolean) {
			if (raiseEvent) loaded++;
			return store.user;
		},
		async removeUser() {
			store.user = null;
		},
		async signinSilent() {
			const result = await renew();
			store.user = result;
			return result;
		},
	};
}

describe("coordinated OIDC renewal", () => {
	test("timer and API recovery share one request and preserve the original error", async () => {
		const result = deferred<User>();
		const started = deferred<void>();
		let calls = 0;
		const state = manager({ user: user("old") }, async () => {
			calls++;
			started.resolve();
			return result.promise;
		});
		coordinateOidcRenewal(state);
		const timer = state.signinSilent();
		const recovery = renewOidcSession({
			events: state.events,
			signinSilent: async () => null,
		});
		await started.promise;
		expect(calls).toBe(1);
		const networkError = new TypeError("Network unavailable");
		result.reject(networkError);
		await expect(timer).rejects.toBe(networkError);
		await expect(recovery).rejects.toBe(networkError);
	});

	test("another manager reuses the rotation saved while waiting for the shared lock", async () => {
		const store = { user: user("old") as User | null };
		const result = deferred<User>();
		const started = deferred<void>();
		const first = manager(
			store,
			async () => {
				started.resolve();
				return result.promise;
			},
			"https://shared.example",
		);
		let secondCalls = 0;
		const second = manager(
			store,
			async () => {
				secondCalls++;
				return user("wrong");
			},
			"https://shared.example",
		);
		coordinateOidcRenewal(first);
		coordinateOidcRenewal(second);
		const a = first.signinSilent();
		await started.promise;
		const b = second.signinSilent();
		await Promise.resolve();
		result.resolve(user("new"));
		expect((await a)?.access_token).toBe("new");
		expect((await b)?.access_token).toBe("new");
		expect(secondCalls).toBe(0);
		expect(second.loaded).toBe(1);
	});

	test("signout wins over a refresh already in flight", async () => {
		const store = { user: user("old") as User | null };
		const result = deferred<User>();
		const started = deferred<void>();
		const state = manager(
			store,
			async () => {
				started.resolve();
				return result.promise;
			},
			"https://signout.example",
		);
		coordinateOidcRenewal(state);
		const pending = state.signinSilent();
		await started.promise;
		const signout = state.removeUser();
		result.resolve(user("new"));
		await expect(pending).rejects.toThrow("session ended");
		await signout;
		expect(store.user).toBeNull();
	});

	test("another tab's sign-in waits for renewal and remains the persisted account", async () => {
		const store = { user: user("old") as User | null };
		const started = deferred<void>();
		const response = deferred<User>();
		const renewing = manager(
			store,
			async () => {
				started.resolve();
				return response.promise;
			},
			"https://callback.example",
		);
		let callbackCalls = 0;
		const signedIn = {
			...user("signed-in"),
			profile: { sub: "other-account" },
		};
		const callback = {
			...manager(store, async () => user("unused"), "https://callback.example"),
			async signinRedirectCallback() {
				callbackCalls++;
				store.user = signedIn;
				return signedIn;
			},
		};
		coordinateOidcRenewal(renewing);
		coordinateOidcRenewal(callback);
		const renewal = renewing.signinSilent();
		await started.promise;
		const signIn = callback.signinRedirectCallback();
		await Promise.resolve();
		expect(callbackCalls).toBe(0);
		response.resolve(user("rotated"));
		await renewal;
		await signIn;
		expect(store.user).toBe(signedIn);
	});

	test("does not return a different account to the caller waiting for renewal", async () => {
		const state = manager(
			{ user: user("old") },
			async () => ({ ...user("new"), profile: { sub: "another-account" } }),
			"https://switch.example",
		);
		coordinateOidcRenewal(state);
		await expect(state.signinSilent()).rejects.toThrow("account changed");
	});

	test("only explicit provider refusals require interactive sign-in", () => {
		expect(requiresInteractiveSignIn({ error: "invalid_grant" })).toBe(true);
		expect(requiresInteractiveSignIn({ error: "login_required" })).toBe(true);
		expect(
			requiresInteractiveSignIn({ error: "temporarily_unavailable" }),
		).toBe(false);
		expect(requiresInteractiveSignIn(new TypeError("Failed to fetch"))).toBe(
			false,
		);
	});
});
