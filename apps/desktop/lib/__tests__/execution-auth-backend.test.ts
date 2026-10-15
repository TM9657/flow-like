import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	fetch: vi.fn(async () => new Response("{}", { status: 200 })),
	invoke: vi
		.fn()
		.mockImplementation(async (command: string) =>
			command === "execution_open_auth_session" ? "native-session" : undefined,
		),
}));
vi.mock("@tauri-apps/plugin-http", () => ({ fetch: mocks.fetch }));
vi.mock("@tauri-apps/api/core", async (importOriginal) => ({
	...(await importOriginal<typeof import("@tauri-apps/api/core")>()),
	invoke: mocks.invoke,
}));
vi.mock("sonner", () => ({
	toast: Object.assign(vi.fn(), {
		success: vi.fn(),
		error: vi.fn(),
		info: vi.fn(),
		warning: vi.fn(),
		dismiss: vi.fn(),
	}),
}));

import { TauriBackend } from "../../components/tauri-provider";

const auth = {
	isAuthenticated: true,
	isLoading: false,
	user: { access_token: "first", profile: { sub: "user-a" } },
};
const hub = "https://first-hub.test";

function updates() {
	return mocks.invoke.mock.calls
		.filter(([command]) => command === "execution_set_auth")
		.map(([, args]) => args);
}

beforeEach(() => mocks.invoke.mockClear());

describe("desktop execution session bridge", () => {
	test("execution awaits renewal and publishes its returned token before React updates", async () => {
		vi.stubGlobal("navigator", { onLine: true });
		try {
			const renewed = { ...auth.user, access_token: "renewed", expired: false };
			const signinSilent = vi.fn(async () => renewed);
			const staleAuth = {
				...auth,
				user: { ...auth.user, expired: true },
				signinSilent,
			};
			const backend = new TauriBackend(() => undefined);
			backend.pushProfile({ hub } as never);
			backend.pushAuthContext(staleAuth as never, hub);
			await backend.prepareExecutionAuth();
			expect(signinSilent).toHaveBeenCalledTimes(1);
			expect(updates().at(-1)).toMatchObject({ token: "renewed" });
			expect(backend.auth?.user?.access_token).toBe("renewed");
			expect(staleAuth.user.access_token).toBe("first");
		} finally {
			vi.unstubAllGlobals();
		}
	});

	test("switching hubs clears HTTP and registry credentials before the next sign-in config arrives", async () => {
		vi.stubGlobal("navigator", { onLine: true });
		try {
			const backend = new TauriBackend(() => undefined);
			backend.pushProfile({ hub } as never);
			backend.pushAuthContext(auth as never, hub);
			await backend.prepareExecutionAuth();
			const nextProfile = { hub: "https://second-hub.test" } as never;
			backend.pushProfile(nextProfile);
			expect(backend.auth).toBeUndefined();
			await backend.apiState.fetch(nextProfile, "user/info");
			const [url, init] = mocks.fetch.mock.calls.at(-1) as unknown as [
				string,
				RequestInit,
			];
			expect(url).toBe("https://second-hub.test/api/v1/user/info");
			expect(new Headers(init.headers).has("Authorization")).toBe(false);
			await expect(
				backend.apiState.fetch(nextProfile, "apps/project/board"),
			).rejects.toThrow("Authentication token required");
			await vi.waitFor(() =>
				expect(
					mocks.invoke.mock.calls
						.filter(([command]) => command === "registry_set_auth_token")
						.at(-1),
				).toEqual(["registry_set_auth_token", { token: null }]),
			);
		} finally {
			vi.unstubAllGlobals();
		}
	});
	test("renewed tokens reach native execution with the original session ID", async () => {
		const backend = new TauriBackend(() => undefined);
		backend.pushProfile({ hub } as never);
		backend.pushAuthContext(auth as never, hub);
		expect(await backend.prepareExecutionAuth()).toBe(hub);
		const sessionId = updates().at(-1).sessionId;
		backend.pushAuthContext(
			{ ...auth, user: { ...auth.user, access_token: "renewed" } } as never,
			hub,
		);
		await backend.prepareExecutionAuth();
		expect(updates().at(-1)).toMatchObject({
			sessionId,
			hub,
			subject: "user-a",
			token: "renewed",
		});
		expect(sessionId).toBe("native-session");
		expect(
			mocks.invoke.mock.calls.filter(
				([command]) => command === "execution_open_auth_session",
			),
		).toHaveLength(1);
	});

	test("sign-out clears the native session and blocks new online runs", async () => {
		const backend = new TauriBackend(() => undefined);
		backend.pushProfile({ hub } as never);
		backend.pushAuthContext(auth as never, hub);
		await backend.prepareExecutionAuth();
		backend.pushAuthContext(
			{ ...auth, isAuthenticated: false, user: null } as never,
			hub,
		);
		await expect(backend.prepareExecutionAuth()).rejects.toThrow("Sign in");
		expect(updates().at(-1)).toMatchObject({ hub, subject: null, token: null });
	});

	test("a profile switch cannot send the previous hub's token to the new hub", async () => {
		const backend = new TauriBackend(() => undefined);
		backend.pushProfile({ hub } as never);
		backend.pushAuthContext(auth as never, hub);
		await backend.prepareExecutionAuth();
		backend.pushProfile({ hub: "https://second-hub.test" } as never);
		backend.pushAuthContext(auth as never, hub);
		await expect(backend.prepareExecutionAuth()).rejects.toThrow("Sign in");
		expect(
			updates()
				.filter((update) => update.hub === "https://second-hub.test")
				.every((update) => update.token === null),
		).toBe(true);
	});
});
