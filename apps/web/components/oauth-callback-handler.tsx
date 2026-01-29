"use client";

import type {
	IOAuthCallbackData,
	IOAuthPendingAuth,
	IOAuthProvider,
} from "@tm9657/flow-like-ui";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";
import { oauthTokenStore } from "../lib/oauth-db";
import { oauthService } from "../lib/oauth-service";

type OAuthCallbackListener = (
	pending: IOAuthPendingAuth,
	token: Awaited<ReturnType<typeof oauthService.handleCallback>>,
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
	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to allow custom deps
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

async function processCallback(payload: IOAuthCallbackData) {
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
		url,
		code,
		state,
		id_token: !!id_token,
		access_token: !!access_token,
		error,
	});

	if (error) {
		const errorMsg = error_description || error;
		console.error("[Web OAuth] Error:", errorMsg);
		toast.error(`Authorization failed: ${errorMsg}`);
		return;
	}

	const isImplicitFlow = !!(access_token || id_token);
	const isCodeFlow = !!code;

	if (!isImplicitFlow && !isCodeFlow) {
		console.error("[Web OAuth] Invalid callback: no code or tokens received");
		toast.error("Invalid callback: no authorization data received");
		return;
	}

	if (!state) {
		console.error("[Web OAuth] Missing state in callback");
		toast.error("Invalid callback: missing state parameter");
		return;
	}

	try {
		const pending = await oauthTokenStore.getPendingAuth(state);

		if (!pending) {
			console.error("[Web OAuth] No pending auth found for state:", state);
			toast.error("Authorization session expired or invalid");
			return;
		}

		const provider = pending.provider ?? providerCache?.get(pending.providerId);

		if (!provider) {
			console.error(
				"[Web OAuth] Provider not found in cache or pending auth:",
				pending.providerId,
			);
			toast.error(`Provider not found: ${pending.providerId}. Please retry.`);
			return;
		}

		let token: Awaited<ReturnType<typeof oauthService.handleCallback>>;

		if (isImplicitFlow) {
			token = await oauthService.handleImplicitCallback(pending, provider, {
				access_token: access_token!,
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

		for (const listener of listeners) {
			try {
				listener(pending, token);
			} catch (e) {
				console.error("[Web OAuth] Callback listener error:", e);
			}
		}
	} catch (e) {
		console.error("[Web OAuth] Failed to handle callback:", e);
		toast.error(
			`Authorization failed: ${e instanceof Error ? e.message : "Unknown error"}`,
		);
	}
}

export function OAuthCallbackHandler({
	children,
}: {
	children: React.ReactNode;
}) {
	useEffect(() => {
		const handleOAuthEvent = (event: Event) => {
			const customEvent = event as CustomEvent<IOAuthCallbackData>;
			const payload = customEvent.detail;
			if (!payload) return;

			console.log("[Web OAuth] Event received:", payload);
			processCallback(payload);
		};

		const checkPendingCallback = () => {
			try {
				const pendingData = sessionStorage.getItem("oauth-callback-pending");
				if (!pendingData) return;

				const data = JSON.parse(pendingData);
				if (Date.now() - data.timestamp > 60000) {
					sessionStorage.removeItem("oauth-callback-pending");
					return;
				}

				sessionStorage.removeItem("oauth-callback-pending");

				console.log("[Web OAuth] Processing pending callback from sessionStorage");
				processCallback({
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
			} catch (e) {
				console.error("[Web OAuth] Failed to process pending callback:", e);
				sessionStorage.removeItem("oauth-callback-pending");
			}
		};

		window.addEventListener("thirdparty-oauth-callback", handleOAuthEvent);

		const timer = setTimeout(checkPendingCallback, 100);

		return () => {
			window.removeEventListener("thirdparty-oauth-callback", handleOAuthEvent);
			clearTimeout(timer);
		};
	}, []);

	return <>{children}</>;
}
