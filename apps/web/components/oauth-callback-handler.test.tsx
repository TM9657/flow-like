// @vitest-environment happy-dom
import "fake-indexeddb/auto";
import { StrictMode, act } from "react";
import { type Root, createRoot } from "react-dom/client";
import {
	afterAll,
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	vi,
} from "vitest";
import {
	configureOAuthSession,
	oauthDB,
	oauthTokenStore,
} from "../../../packages/ui/db/oauth-db";
import type { OAuthSession } from "../../../packages/ui/lib/oauth/scope";
import { createOAuthService } from "../../../packages/ui/lib/oauth/service";
import type {
	IOAuthCallbackData,
	IOAuthProvider,
} from "../../../packages/ui/lib/oauth/types";
import {
	OAUTH_CALLBACK_CHANNEL,
	readPendingOAuthCallback,
	storePendingOAuthCallback,
} from "../lib/oauth-callback-storage";

const harness = vi.hoisted(() => ({
	ready: false,
	service: null as ReturnType<typeof createOAuthService> | null,
	success: vi.fn(),
	error: vi.fn(),
	httpPost: vi.fn(),
}));
vi.mock("./auth-provider", () => ({
	useOAuthCallbackReady: () => harness.ready,
}));
vi.mock(
	"../lib/oauth-db",
	async () => import("../../../packages/ui/db/oauth-db"),
);
vi.mock("../lib/oauth-service", () => ({
	getOAuthService: () => harness.service,
}));
vi.mock("@flow-like/locales", () => ({
	i18n: { t: (_key: string, fallback: string) => fallback },
}));
vi.mock("sonner", () => ({
	toast: { success: harness.success, error: harness.error },
}));

import {
	OAuthCallbackHandler,
	addOAuthCallbackListener,
} from "./oauth-callback-handler";

const provider: IOAuthProvider = {
	id: "google",
	name: "Google",
	client_id: "google-client",
	auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
	token_url: "https://oauth2.googleapis.com/token",
	scopes: ["openid", "email", "profile"],
	pkce_required: true,
	requires_secret_proxy: true,
	use_device_flow: false,
};
let owner: OAuthSession;
let session: OAuthSession | undefined;
let payload: IOAuthCallbackData & { state: string };
let root: Root;
let container: HTMLDivElement;

async function render(ready = false) {
	harness.ready = ready;
	await act(async () => {
		root.render(
			<StrictMode>
				<OAuthCallbackHandler>Callback page</OAuthCallbackHandler>
			</StrictMode>,
		);
	});
}

function dispatchCallback() {
	window.dispatchEvent(
		new CustomEvent("thirdparty-oauth-callback", { detail: payload }),
	);
}

beforeEach(async () => {
	(
		globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
	).IS_REACT_ACT_ENVIRONMENT = true;
	vi.clearAllMocks();
	vi.stubGlobal("BroadcastChannel", undefined);
	harness.ready = false;
	harness.httpPost.mockResolvedValue(
		new Response(JSON.stringify({ access_token: "google-access-token" })),
	);
	localStorage.clear();
	sessionStorage.clear();
	await oauthDB.scopedPendingAuth.clear();
	await oauthDB.scopedTokens.clear();
	owner = {
		hub: "https://hub.test",
		issuer: "https://login.test",
		subject: crypto.randomUUID(),
	};
	session = owner;
	configureOAuthSession(() => session);
	harness.service = createOAuthService({
		tokenStore: oauthTokenStore,
		platform: "web-prod",
		runtime: {
			openUrl: async () => {},
			httpPost: harness.httpPost,
			httpGet: async () => new Response("{}"),
		},
	});
	const { state } = await harness.service.startAuthorization(provider);
	payload = {
		url: `https://flow-like.com/thirdparty/callback?code=google-code&state=${state}`,
		code: "google-code",
		state,
		id_token: null,
		access_token: null,
		token_type: null,
		expires_in: null,
		scope: null,
		error: null,
		error_description: null,
	};
	session = undefined;
	container = document.createElement("div");
	document.body.appendChild(container);
	root = createRoot(container);
});

afterEach(async () => {
	await act(async () => root.unmount());
	container.remove();
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});
afterAll(async () => {
	await oauthDB.delete();
});

describe("OAuth callback session initialization", () => {
	it("retains the stored redirect until the Hub session and profile are ready", async () => {
		storePendingOAuthCallback(payload);
		await render();
		dispatchCallback();
		expect(harness.httpPost).not.toHaveBeenCalled();
		expect(harness.error).not.toHaveBeenCalled();
		expect(readPendingOAuthCallback()?.state).toBe(payload.state);

		const listener = vi.fn();
		const unsubscribe = addOAuthCallbackListener(listener);
		try {
			session = owner;
			await render(true);
			await vi.waitFor(() => expect(harness.success).toHaveBeenCalledOnce());
			expect(harness.httpPost).toHaveBeenCalledOnce();
			expect(listener).toHaveBeenCalledOnce();
			expect(harness.error).not.toHaveBeenCalled();
			expect(readPendingOAuthCallback()).toBeNull();
			expect((await oauthTokenStore.getToken(provider.id))?.access_token).toBe(
				"google-access-token",
			);
		} finally {
			unsubscribe();
		}
	});

	it("retains callback events during initialization even when storage is unavailable", async () => {
		await render();
		dispatchCallback();
		expect(harness.error).not.toHaveBeenCalled();
		session = owner;
		await render(true);
		await vi.waitFor(() => expect(harness.success).toHaveBeenCalledOnce());
		expect(harness.httpPost).toHaveBeenCalledOnce();
	});

	it("keeps a queued callback while profile loading exceeds the storage retention time", async () => {
		storePendingOAuthCallback(payload);
		await render();
		vi.spyOn(Date, "now").mockReturnValue(Date.now() + 61_000);
		session = owner;
		await render(true);
		await vi.waitFor(() => expect(harness.success).toHaveBeenCalledOnce());
		expect(harness.httpPost).toHaveBeenCalledOnce();
		expect(harness.error).not.toHaveBeenCalled();
	});

	it("keeps cross-tab completion notifications working through readiness changes", async () => {
		const channels: EventTarget[] = [];
		vi.stubGlobal(
			"BroadcastChannel",
			class extends EventTarget {
				constructor(name: string) {
					super();
					expect(name).toBe(OAUTH_CALLBACK_CHANNEL);
					channels.push(this);
				}
				close() {}
			},
		);
		const listener = vi.fn();
		const unsubscribe = addOAuthCallbackListener(listener);
		const completion = {
			type: "oauth-complete",
			payload: {
				pending: await oauthDB.scopedPendingAuth.get(payload.state),
				token: {
					providerId: provider.id,
					access_token: "google-access-token",
					scopes: provider.scopes,
					storedAt: Date.now(),
				},
				timestamp: Date.now(),
			},
		};
		const receiveCompletion = () => {
			channels
				.at(-1)
				?.dispatchEvent(new MessageEvent("message", { data: completion }));
		};
		try {
			await render();
			receiveCompletion();
			expect(listener).toHaveBeenCalledOnce();
			session = owner;
			await render(true);
			receiveCompletion();
			expect(listener).toHaveBeenCalledOnce();
			expect(harness.httpPost).not.toHaveBeenCalled();
		} finally {
			unsubscribe();
		}
	});

	it("does not consume a callback again when readiness changes during exchange", async () => {
		let finish!: (response: Response) => void;
		harness.httpPost.mockImplementation(
			() =>
				new Promise<Response>((resolve) => {
					finish = resolve;
				}),
		);
		storePendingOAuthCallback(payload);
		session = owner;
		await render(true);
		await vi.waitFor(() => expect(harness.httpPost).toHaveBeenCalledOnce());
		await render(false);
		dispatchCallback();
		await render(true);
		dispatchCallback();
		finish(
			new Response(JSON.stringify({ access_token: "google-access-token" })),
		);
		await vi.waitFor(() => expect(harness.success).toHaveBeenCalledOnce());
		dispatchCallback();
		expect(harness.httpPost).toHaveBeenCalledOnce();
		expect(harness.error).not.toHaveBeenCalled();
	});

	it.each(["expired", "other-account", "signed-out"])(
		"still rejects a %s callback after initialization",
		async (scenario) => {
			session = scenario === "signed-out" ? undefined : owner;
			if (scenario === "other-account") {
				session = { ...owner, subject: "another-user" };
			}
			if (scenario === "expired") {
				await oauthDB.scopedPendingAuth.update(payload.state, {
					initiatedAt: Date.now() - 600001,
				});
			}
			storePendingOAuthCallback(payload);
			await render(true);
			await vi.waitFor(() =>
				expect(harness.error).toHaveBeenCalledWith(
					"Authorization session expired or invalid",
				),
			);
			expect(harness.httpPost).not.toHaveBeenCalled();
			expect(harness.success).not.toHaveBeenCalled();
			expect(await oauthDB.scopedPendingAuth.get(payload.state)).toBeDefined();
		},
	);
});
