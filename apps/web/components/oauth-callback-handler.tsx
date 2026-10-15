"use client";

import type {
	IOAuthCallbackData,
	IOAuthPendingAuth,
	IOAuthProvider,
	IStoredOAuthToken,
	OAuthService,
} from "@flow-like/flow-like-ui";
import { i18n as i18next } from "@flow-like/locales";
import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import {
	type IStoredOAuthCallbackCompletion,
	OAUTH_CALLBACK_CHANNEL,
	broadcastOAuthCallbackCompletion,
	clearPendingOAuthCallback,
	parseOAuthCallbackCompletion,
	readPendingOAuthCallback,
} from "../lib/oauth-callback-storage";
import { oauthTokenStore } from "../lib/oauth-db";
import { getOAuthService } from "../lib/oauth-service";
import { useOAuthCallbackReady } from "./auth-provider";

type OAuthCallbackListener = (
	pending: IOAuthPendingAuth,
	token: Awaited<ReturnType<OAuthService["handleCallback"]>>,
) => void;

const listeners = new Set<OAuthCallbackListener>();

export function addOAuthCallbackListener(listener: OAuthCallbackListener) {
	listeners.add(listener);
	return () => listeners.delete(listener);
}

export function useOAuthCallbackListener(
	callback: OAuthCallbackListener,
	deps: React.DependencyList = [],
) {
	const memoizedCallback = useCallback(callback, deps);

	useEffect(() => {
		const unsubscribe = addOAuthCallbackListener(memoizedCallback);
		return () => {
			unsubscribe();
		};
	}, [memoizedCallback]);
}

let providerCache: Map<string, IOAuthProvider> | null = null;

export function setProviderCache(providers: Map<string, IOAuthProvider>) {
	providerCache = providers;
}

export function clearProviderCache() {
	providerCache = null;
}

function notifyListeners(pending: IOAuthPendingAuth, token: IStoredOAuthToken) {
	for (const listener of listeners) {
		try {
			listener(pending, token);
		} catch (e) {
			console.error("[Web OAuth] Callback listener error:", e);
		}
	}
}

function getCompletionKey(completion: IStoredOAuthCallbackCompletion): string {
	return [
		completion.pending.providerId,
		completion.pending.state,
		completion.token.providerId,
		completion.token.storedAt,
		completion.timestamp,
	].join(":");
}

async function processCallback(payload: IOAuthCallbackData): Promise<boolean> {
	const {
		url,
		code,
		state,
		id_token,
		access_token,
		token_type,
		expires_in,
		scope,
		error,
		error_description,
	} = payload;

	console.log("[Web OAuth] Callback received:", {
		hasAuthorizationCode: !!code,
		hasIdToken: !!id_token,
		hasAccessToken: !!access_token,
		error,
	});

	if (error) {
		const errorMsg = error_description || error;
		console.error("[Web OAuth] Callback error:", error);
		toast.error(`Authorization failed: ${errorMsg}`);
		return false;
	}

	const isImplicitFlow = !!(access_token || id_token);
	const isCodeFlow = !!code;

	if (!isImplicitFlow && !isCodeFlow) {
		console.error("[Web OAuth] Invalid callback: no code or tokens received");
		toast.error("Invalid callback: no authorization data received");
		return false;
	}

	if (!state) {
		console.error("[Web OAuth] Missing state in callback");
		toast.error("Invalid callback: missing state parameter");
		return false;
	}

	try {
		const pending = await oauthTokenStore.getPendingAuth(state);

		if (!pending) {
			console.error("[Web OAuth] No pending auth found for callback state");
			toast.error("Authorization session expired or invalid");
			return false;
		}

		const provider = pending.provider ?? providerCache?.get(pending.providerId);

		if (!provider) {
			console.error(
				"[Web OAuth] Provider not found in cache or pending auth:",
				pending.providerId,
			);
			toast.error(`Provider not found: ${pending.providerId}. Please retry.`);
			return false;
		}

		const oauthService = getOAuthService(pending.apiBaseUrl);
		let token: Awaited<ReturnType<OAuthService["handleCallback"]>>;

		if (isImplicitFlow) {
			if (!access_token) {
				console.error(
					"[Web OAuth] Invalid implicit callback: missing access token",
				);
				toast.error("Invalid callback: missing access token");
				return false;
			}

			token = await oauthService.handleImplicitCallback(pending, provider, {
				access_token,
				id_token: id_token ?? undefined,
				token_type: token_type ?? "Bearer",
				expires_in: expires_in ? Number.parseInt(expires_in, 10) : undefined,
				scope: scope ?? undefined,
			});
		} else {
			token = await oauthService.handleCallback(url, provider);
		}

		console.log("[Web OAuth] Token obtained for provider:", provider.name);
		toast.success(`Connected to ${provider.name}`);

		notifyListeners(pending, token as IStoredOAuthToken);
		broadcastOAuthCallbackCompletion(pending, token as IStoredOAuthToken);

		return true;
	} catch (e) {
		console.error("[Web OAuth] Failed to handle callback");
		toast.error(
			i18next.t("authorizationFailedVal", "Authorization failed: {{val}}", {
				val: e instanceof Error ? e.message : "Unknown error",
			}),
		);
		return false;
	}
}

export function OAuthCallbackHandler({
	children,
}: {
	children: React.ReactNode;
}) {
	const callbackReady = useOAuthCallbackReady();
	const seenCompletionKeys = useRef(new Set<string>());
	const processingCallbackStates = useRef(new Set<string>());
	const queuedCallbacks = useRef(new Map<string, IOAuthCallbackData>());

	useEffect(() => {
		let channel: BroadcastChannel | null = null;

		const processCallbackOnce = (payload: IOAuthCallbackData) => {
			const key = payload.state ?? payload.url;
			if (!callbackReady) {
				queuedCallbacks.current.set(key, payload);
				return;
			}
			queuedCallbacks.current.delete(key);

			const callbackState = payload.state;
			if (
				callbackState &&
				processingCallbackStates.current.has(callbackState)
			) {
				return;
			}
			if (callbackState) {
				processingCallbackStates.current.add(callbackState);
			}

			void processCallback(payload).then((processed) => {
				if (processed) {
					clearPendingOAuthCallback();
					return;
				}

				if (callbackState) {
					processingCallbackStates.current.delete(callbackState);
				}
			});
		};

		const handleCompletion = (rawValue: string | null) => {
			if (!rawValue) {
				return;
			}

			const completion = parseOAuthCallbackCompletion(rawValue);
			if (!completion) {
				return;
			}

			const completionKey = getCompletionKey(completion);
			if (seenCompletionKeys.current.has(completionKey)) {
				return;
			}
			seenCompletionKeys.current.add(completionKey);

			console.log(
				"[Web OAuth] Received cross-tab OAuth completion for provider:",
				completion.pending.providerId,
			);
			notifyListeners(completion.pending, completion.token);
		};

		const handleOAuthEvent = (event: Event) => {
			const customEvent = event as CustomEvent<IOAuthCallbackData>;
			const payload = customEvent.detail;
			if (!payload) return;

			console.log("[Web OAuth] Event received");
			processCallbackOnce(payload);
		};

		const checkPendingCallback = () => {
			const data = readPendingOAuthCallback();
			if (!data) {
				return;
			}

			console.log("[Web OAuth] Processing pending callback from storage");
			processCallbackOnce({
				url: data.url,
				code: data.code,
				state: data.state,
				id_token: data.id_token,
				access_token: data.access_token,
				token_type: data.token_type,
				expires_in: data.expires_in,
				scope: data.scope,
				error: null,
				error_description: null,
			});
		};

		const handleChannelMessage = (event: MessageEvent) => {
			if (event.data?.type !== "oauth-complete") {
				return;
			}

			handleCompletion(JSON.stringify(event.data.payload));
		};

		window.addEventListener("thirdparty-oauth-callback", handleOAuthEvent);

		try {
			channel = new BroadcastChannel(OAUTH_CALLBACK_CHANNEL);
			channel.addEventListener("message", handleChannelMessage);
		} catch {
			channel = null;
		}

		// Callback pages mount before the Hub session and profile have loaded.
		// Keep their payload until the authenticated session can validate its owner.
		if (callbackReady) {
			for (const payload of queuedCallbacks.current.values()) {
				processCallbackOnce(payload);
			}
		}
		checkPendingCallback();

		return () => {
			window.removeEventListener("thirdparty-oauth-callback", handleOAuthEvent);
			if (channel) {
				channel.removeEventListener("message", handleChannelMessage);
				channel.close();
			}
		};
	}, [callbackReady]);

	return <>{children}</>;
}
