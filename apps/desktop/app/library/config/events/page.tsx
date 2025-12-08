"use client";
import type { IOAuthProvider, IStoredOAuthToken } from "@tm9657/flow-like-ui";
import EventsPage from "@tm9657/flow-like-ui/components/settings/events/events-page";
import { useCallback } from "react";
import { EVENT_CONFIG } from "../../../../lib/event-config";
import { oauthConsentStore, oauthTokenStore } from "../../../../lib/oauth-db";
import { oauthService } from "../../../../lib/oauth-service";

export default function Page() {
	const handleStartOAuth = useCallback(async (provider: IOAuthProvider) => {
		await oauthService.startAuthorization(provider);
	}, []);

	const handleRefreshToken = useCallback(
		async (provider: IOAuthProvider, token: IStoredOAuthToken) => {
			return oauthService.refreshToken(provider, token);
		},
		[],
	);

	return (
		<EventsPage
			eventMapping={EVENT_CONFIG}
			tokenStore={oauthTokenStore}
			consentStore={oauthConsentStore}
			onStartOAuth={handleStartOAuth}
			onRefreshToken={handleRefreshToken}
		/>
	);
}
