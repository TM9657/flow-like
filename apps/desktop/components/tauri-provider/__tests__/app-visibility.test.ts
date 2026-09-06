import { IAppVisibility } from "@flow-like/flow-like-ui";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	invoke: vi.fn(),
	fetcher: vi.fn(),
	visibilityGet: vi.fn(),
	visibilityPut: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", async (importOriginal) => ({
	...(await importOriginal<typeof import("@tauri-apps/api/core")>()),
	invoke: mocks.invoke,
}));

vi.mock("../../../lib/api", () => ({
	fetcher: mocks.fetcher,
	streamFetcher: vi.fn(),
}));

vi.mock("../../../lib/apps-db", () => ({
	appsDB: {
		visibility: { get: mocks.visibilityGet, put: mocks.visibilityPut },
		shortcuts: {},
	},
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

import { TauriBackend } from "../../tauri-provider";

const APP = "app-1";

function backend() {
	return new TauriBackend(
		() => undefined,
		undefined,
		{
			isAuthenticated: true,
			user: { access_token: "token", profile: { sub: "user-1" } },
		} as never,
		{ id: "profile-1", hub: "hub-1" } as never,
	);
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.visibilityGet.mockResolvedValue(undefined);
	mocks.visibilityPut.mockResolvedValue(undefined);
});

describe("app visibility resolution", () => {
	test("a stated visibility is used and cached", async () => {
		mocks.invoke.mockResolvedValue({ visibility: IAppVisibility.Offline });

		await expect(backend().isLocalOnly(APP)).resolves.toBe(true);
		expect(mocks.visibilityPut).toHaveBeenCalledWith({
			visibility: IAppVisibility.Offline,
			appId: APP,
		});
	});

	// The regression this suite exists for: a hosted app this device has never
	// opened has no local manifest, so `get_app` fails. Guessing "offline" and
	// persisting that guess makes `isLocalOnly` claim the app is local-only,
	// which closes the only path that downloads its boards.
	test("an app with no local manifest stays unknown, not local-only", async () => {
		mocks.invoke.mockRejectedValue(new Error("App not found"));

		await expect(backend().isLocalOnly(APP)).resolves.toBe(false);
		expect(mocks.visibilityPut).not.toHaveBeenCalled();
	});

	test("a manifest that states no visibility is not cached as offline", async () => {
		mocks.invoke.mockResolvedValue({});

		await expect(backend().isLocalOnly(APP)).resolves.toBe(false);
		expect(mocks.visibilityPut).not.toHaveBeenCalled();
	});

	// What fork_app / acquire_app call so a brand-new app is classified before
	// anything reads its boards, instead of depending on a manifest that has not
	// been written to this device yet.
	test("a recorded visibility answers without consulting the manifest", async () => {
		const target = backend();
		await target.appState.recordLocalAppVisibility?.(
			APP,
			IAppVisibility.Private,
		);
		expect(mocks.visibilityPut).toHaveBeenCalledWith({
			visibility: IAppVisibility.Private,
			appId: APP,
		});

		mocks.visibilityGet.mockResolvedValue({
			appId: APP,
			visibility: IAppVisibility.Private,
		});

		await expect(target.isLocalOnly(APP)).resolves.toBe(false);
		await expect(target.isOffline(APP)).resolves.toBe(false);
		expect(mocks.invoke).not.toHaveBeenCalledWith("get_app", { appId: APP });
	});

	test("an authoritative hosted App read does not fall back to or update local cache", async () => {
		const remote = { id: APP, visibility: IAppVisibility.Private };
		mocks.invoke.mockRejectedValueOnce(new Error("No local manifest"));
		mocks.fetcher.mockResolvedValueOnce(remote);

		await expect(backend().appState.getAppAuthoritative(APP)).resolves.toBe(
			remote,
		);
		expect(mocks.fetcher).toHaveBeenCalledWith(
			{ id: "profile-1", hub: "hub-1" },
			`apps/${APP}`,
			{ method: "GET" },
			expect.objectContaining({ isAuthenticated: true }),
		);
		expect(mocks.invoke).not.toHaveBeenCalledWith(
			"update_app",
			expect.anything(),
		);
		expect(mocks.visibilityPut).not.toHaveBeenCalled();
	});

	test("an authoritative local-only App read stays native", async () => {
		const local = { id: APP, visibility: IAppVisibility.Offline };
		mocks.visibilityGet.mockResolvedValueOnce({
			appId: APP,
			visibility: IAppVisibility.Offline,
		});
		mocks.invoke.mockResolvedValueOnce(local);

		await expect(backend().appState.getAppAuthoritative(APP)).resolves.toBe(
			local,
		);
		expect(mocks.invoke).toHaveBeenCalledWith("get_app", { appId: APP });
		expect(mocks.fetcher).not.toHaveBeenCalled();
	});

	test("an authoritative hosted App update never follows the offline guess", async () => {
		const app = { id: APP, visibility: IAppVisibility.Private };
		mocks.invoke.mockRejectedValueOnce(new Error("No local manifest"));
		mocks.fetcher.mockResolvedValueOnce(undefined);

		await backend().appState.updateAppAuthoritative(app as never);
		expect(mocks.fetcher).toHaveBeenCalledWith(
			{ id: "profile-1", hub: "hub-1" },
			`apps/${APP}`,
			{
				method: "PUT",
				body: JSON.stringify({ app }),
			},
			expect.objectContaining({ isAuthenticated: true }),
		);
		expect(mocks.invoke).not.toHaveBeenCalledWith(
			"update_app",
			expect.anything(),
		);
	});

	test("an authoritative local-only App update stays native", async () => {
		const app = { id: APP, visibility: IAppVisibility.Offline };
		mocks.visibilityGet.mockResolvedValueOnce({
			appId: APP,
			visibility: IAppVisibility.Offline,
		});

		await backend().appState.updateAppAuthoritative(app as never);
		expect(mocks.invoke).toHaveBeenCalledWith("update_app", { app });
		expect(mocks.fetcher).not.toHaveBeenCalled();
	});

	test("an unknown-visibility App build uses durable remote storage", async () => {
		const record = { app_id: APP, build_id: "build-1", revision: 0 };
		mocks.invoke.mockRejectedValueOnce(new Error("No local manifest"));
		mocks.fetcher.mockResolvedValueOnce(record);

		await expect(backend().appState.readAppBuild(APP, "build-1")).resolves.toBe(
			record,
		);
		expect(mocks.fetcher.mock.calls[0][1]).toBe(
			`apps/${APP}/flowpilot-builds/build-1`,
		);
		expect(mocks.invoke).not.toHaveBeenCalledWith(
			"read_app_build",
			expect.anything(),
		);
	});
});
