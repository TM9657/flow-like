import {
	type OAuthPlatform,
	type OAuthService,
	createOAuthService,
} from "@tm9657/flow-like-ui";
import { oauthTokenStore } from "./oauth-db";
import { tauriOAuthRuntime } from "./tauri-oauth-runtime";

export type { IOAuthProvider } from "@tm9657/flow-like-ui";

let cachedService: OAuthService | null = null;
let cachedApiBaseUrl: string | undefined;

function getWebPlatform(): OAuthPlatform {
	if (typeof window === "undefined") {
		return "web-prod";
	}
	// Check if running on localhost for dev environment
	if (
		window.location.hostname === "localhost" ||
		window.location.hostname === "127.0.0.1"
	) {
		return "web-dev";
	}
	return "web-prod";
}

export function getOAuthService(apiBaseUrl?: string): OAuthService {
	// Always recalculate platform at runtime to ensure correct value
	const platform = getWebPlatform();

	if (cachedService && cachedApiBaseUrl === apiBaseUrl) {
		return cachedService;
	}

	cachedApiBaseUrl = apiBaseUrl;
	cachedService = createOAuthService({
		runtime: tauriOAuthRuntime,
		tokenStore: oauthTokenStore,
		getApiBaseUrl: apiBaseUrl ? async () => apiBaseUrl : undefined,
		platform,
	});

	return cachedService;
}

// Lazy getter for default service (avoids SSR issues with window)
let _oauthService: OAuthService | null = null;
export const oauthService: OAuthService = new Proxy({} as OAuthService, {
	get(_target, prop) {
		if (!_oauthService) {
			_oauthService = createOAuthService({
				runtime: tauriOAuthRuntime,
				tokenStore: oauthTokenStore,
				platform: getWebPlatform(),
			});
		}
		return (_oauthService as any)[prop];
	},
});
