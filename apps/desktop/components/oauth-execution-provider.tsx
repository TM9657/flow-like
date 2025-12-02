"use client";

import {
	OAuthExecutionProvider as BaseOAuthExecutionProvider,
	type IOAuthProvider,
	useOAuthExecutionContext,
} from "@tm9657/flow-like-ui";
import { type ReactNode, useRef } from "react";
import { oauthConsentStore, oauthTokenStore } from "../lib/oauth-db";
import { oauthService } from "../lib/oauth-service";
import { tauriOAuthRuntime } from "../lib/tauri-oauth-runtime";
import {
	clearProviderCache,
	setProviderCache,
	useOAuthCallbackListener,
} from "./oauth-callback-handler";

// Re-export the hook for convenience
export { useOAuthExecutionContext as useOAuthExecution };

export function OAuthExecutionProvider({ children }: { children: ReactNode }) {
	const providerCacheRef = useRef<Map<string, IOAuthProvider>>(new Map());

	// Handle OAuth callback from Tauri deep links
	useOAuthCallbackListener((pending, _token) => {
		// The base provider will update authorizedProviders via onOAuthCallback
	}, []);

	// Sync provider cache with the OAuth callback handler
	const handleProviderCacheUpdate = () => {
		if (providerCacheRef.current.size > 0) {
			setProviderCache(providerCacheRef.current);
		} else {
			clearProviderCache();
		}
	};

	return (
		<BaseOAuthExecutionProvider
			oauthService={oauthService}
			runtime={tauriOAuthRuntime}
			tokenStore={oauthTokenStore}
			consentStore={oauthConsentStore}
			providerCacheRef={providerCacheRef}
			onOAuthCallback={(providerId, token) => {
				handleProviderCacheUpdate();
			}}
		>
			<OAuthCallbackSync providerCacheRef={providerCacheRef}>
				{children}
			</OAuthCallbackSync>
		</BaseOAuthExecutionProvider>
	);
}

// Internal component to sync OAuth callbacks with the provider
function OAuthCallbackSync({
	children,
	providerCacheRef,
}: {
	children: ReactNode;
	providerCacheRef: React.MutableRefObject<Map<string, IOAuthProvider>>;
}) {
	// Keep provider cache in sync
	useOAuthCallbackListener(
		(pending, _token) => {
			if (providerCacheRef.current.size > 0) {
				setProviderCache(providerCacheRef.current);
			}
		},
		[providerCacheRef],
	);

	return <>{children}</>;
}
