// @vitest-environment happy-dom
import type {
	IOAuthCallbackData,
	IOAuthPendingAuth,
	IOAuthProvider,
} from "@flow-like/flow-like-ui";
import { act, createElement } from "react";
import { type Root, createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	callback: undefined as
		| ((event: { payload: IOAuthCallbackData }) => Promise<void>)
		| undefined,
	getPendingAuth: vi.fn(),
	waitForPackageSession: vi.fn(),
	handleCallback: vi.fn(),
	handleImplicitCallback: vi.fn(),
	success: vi.fn(),
	error: vi.fn(),
}));

vi.mock("@flow-like/flow-like-ui", () => ({
	useBackendStore: {
		getState: () => ({
			backend: { waitForPackageSession: mocks.waitForPackageSession },
		}),
	},
}));
vi.mock("@flow-like/locales", () => ({
	i18n: {
		t: (_key: string, template: string, values: { val: string }) =>
			template.replace("{{val}}", values.val),
	},
}));
vi.mock("@tauri-apps/api/event", () => ({
	listen: async (_name: string, callback: typeof mocks.callback) => {
		mocks.callback = callback;
		return () => {
			mocks.callback = undefined;
		};
	},
}));
vi.mock("sonner", () => ({
	toast: { success: mocks.success, error: mocks.error },
}));
vi.mock("../../lib/oauth-db", () => ({
	oauthTokenStore: { getPendingAuth: mocks.getPendingAuth },
}));
vi.mock("../../lib/oauth-service", () => ({
	oauthService: {
		handleCallback: mocks.handleCallback,
		handleImplicitCallback: mocks.handleImplicitCallback,
	},
}));

import {
	OAuthCallbackHandler,
	addOAuthCallbackListener,
} from "../oauth-callback-handler";

const provider = { id: "google", name: "Google" } as IOAuthProvider;
let root: Root;
let container: HTMLDivElement;
let state: string;
let pending: IOAuthPendingAuth;

const payload = (): IOAuthCallbackData => ({
	url: `flow-like://thirdparty/callback?code=code&state=${state}`,
	state,
	code: "code",
	id_token: null,
	access_token: null,
	token_type: null,
	expires_in: null,
	scope: null,
	error: null,
	error_description: null,
});

const dispatch = (data = payload()) => {
	if (!mocks.callback)
		throw new Error("OAuth callback listener is not mounted");
	return mocks.callback({ payload: data });
};

beforeEach(async () => {
	(
		globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
	).IS_REACT_ACT_ENVIRONMENT = true;
	vi.clearAllMocks();
	state = crypto.randomUUID();
	pending = {
		state,
		providerId: provider.id,
		provider,
		codeVerifier: "verifier",
		redirectUri: "flow-like://thirdparty/callback",
		scopes: [],
		initiatedAt: Date.now(),
	};
	mocks.getPendingAuth.mockResolvedValue(pending);
	mocks.waitForPackageSession.mockResolvedValue(undefined);
	mocks.handleCallback.mockResolvedValue({ access_token: "token" });
	mocks.handleImplicitCallback.mockResolvedValue({ access_token: "token" });
	container = document.createElement("div");
	document.body.appendChild(container);
	root = createRoot(container);
	await act(async () =>
		root.render(createElement(OAuthCallbackHandler, null, null)),
	);
});

afterEach(async () => {
	await act(async () => root.unmount());
	container.remove();
});

it("exchanges a code once when native callbacks arrive concurrently", async () => {
	let resolvePending!: (value: IOAuthPendingAuth) => void;
	mocks.getPendingAuth.mockImplementationOnce(
		() =>
			new Promise<IOAuthPendingAuth>((resolve) => {
				resolvePending = resolve;
			}),
	);
	const listener = vi.fn();
	const unsubscribe = addOAuthCallbackListener(listener);
	try {
		const first = dispatch();
		const duplicate = dispatch();
		await Promise.resolve();
		resolvePending(pending);
		await Promise.all([first, duplicate]);
		expect(mocks.getPendingAuth).toHaveBeenCalledOnce();
		expect(mocks.handleCallback).toHaveBeenCalledOnce();
		expect(listener).toHaveBeenCalledOnce();
		expect(mocks.success).toHaveBeenCalledOnce();
		expect(mocks.error).not.toHaveBeenCalled();
	} finally {
		unsubscribe();
	}
});

it("waits for the restored Hub session before reading a startup callback", async () => {
	let restoreSession!: () => void;
	mocks.waitForPackageSession.mockImplementationOnce(
		() =>
			new Promise<void>((resolve) => {
				restoreSession = resolve;
			}),
	);
	const callback = dispatch();
	await Promise.resolve();
	expect(mocks.getPendingAuth).not.toHaveBeenCalled();
	expect(mocks.handleCallback).not.toHaveBeenCalled();
	expect(mocks.error).not.toHaveBeenCalled();
	restoreSession();
	await callback;
	expect(mocks.getPendingAuth).toHaveBeenCalledWith(state);
	expect(mocks.handleCallback).toHaveBeenCalledOnce();
	expect(mocks.error).not.toHaveBeenCalled();
});

it("reports a session initialization failure and allows retry after recovery", async () => {
	mocks.waitForPackageSession.mockRejectedValueOnce(
		new Error("Sign-in initialization timed out"),
	);
	await dispatch();
	expect(mocks.getPendingAuth).not.toHaveBeenCalled();
	expect(mocks.handleCallback).not.toHaveBeenCalled();
	expect(mocks.error).toHaveBeenCalledWith(
		"Authorization failed: Sign-in initialization timed out",
	);
	await dispatch();
	expect(mocks.getPendingAuth).toHaveBeenCalledOnce();
	expect(mocks.handleCallback).toHaveBeenCalledOnce();
});

it("ignores a completed native callback after the handler remounts", async () => {
	await dispatch();
	mocks.getPendingAuth.mockResolvedValue(undefined);
	await act(async () => root.render(null));
	await act(async () =>
		root.render(createElement(OAuthCallbackHandler, null, null)),
	);
	await dispatch();
	expect(mocks.getPendingAuth).toHaveBeenCalledOnce();
	expect(mocks.handleCallback).toHaveBeenCalledOnce();
	expect(mocks.error).not.toHaveBeenCalled();
});

it("ignores a completed implicit callback", async () => {
	const implicit = { ...payload(), code: null, access_token: "token" };
	await dispatch(implicit);
	mocks.getPendingAuth.mockResolvedValue(undefined);
	await dispatch(implicit);
	expect(mocks.handleImplicitCallback).toHaveBeenCalledOnce();
	expect(mocks.handleCallback).not.toHaveBeenCalled();
	expect(mocks.error).not.toHaveBeenCalled();
});

it("allows a callback to be retried after the pending session becomes available", async () => {
	mocks.getPendingAuth.mockResolvedValueOnce(undefined);
	await dispatch();
	expect(mocks.error).toHaveBeenCalledWith(
		"Authorization session expired or invalid",
	);
	await dispatch();
	expect(mocks.getPendingAuth).toHaveBeenCalledTimes(2);
	expect(mocks.handleCallback).toHaveBeenCalledOnce();
});

it("does not suppress a different authorization state", async () => {
	await dispatch();
	state = crypto.randomUUID();
	mocks.getPendingAuth.mockResolvedValue({ ...pending, state });
	await dispatch();
	expect(mocks.getPendingAuth).toHaveBeenCalledTimes(2);
	expect(mocks.handleCallback).toHaveBeenCalledTimes(2);
	expect(mocks.success).toHaveBeenCalledTimes(2);
});
