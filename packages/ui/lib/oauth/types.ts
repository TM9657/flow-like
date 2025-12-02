/**
 * OAuth provider configuration from nodes.
 * This matches the Rust OAuthProvider struct.
 */
export interface IOAuthProvider {
	id: string;
	name: string;
	auth_url: string;
	token_url: string;
	client_id: string;
	/** Client secret for providers that require it (e.g., Notion). Leave empty for PKCE-based flows. */
	client_secret?: string;
	scopes: string[];
	pkce_required: boolean;
	revoke_url?: string;
	userinfo_url?: string;
	oidc_discovery_url?: string;
	jwks_url?: string;
	audience?: string;
	/** Device authorization endpoint URL for device flow (RFC 8628) */
	device_auth_url?: string;
	/** Whether to use device flow instead of standard authorization code flow */
	use_device_flow: boolean;
	/** Additional scopes merged from nodes that require this provider */
	merged_scopes?: string[];
}

/**
 * Device flow authorization response from the device authorization endpoint
 */
export interface IDeviceAuthResponse {
	device_code: string;
	user_code: string;
	verification_uri: string;
	verification_uri_complete?: string;
	expires_in: number;
	interval: number;
}

/**
 * OAuth token stored/passed to backend.
 * This matches the Rust OAuthToken struct.
 */
export interface IOAuthToken {
	access_token: string;
	refresh_token?: string;
	expires_at?: number;
	token_type?: string;
}

/**
 * Extended OAuth token with additional metadata for storage.
 */
export interface IStoredOAuthToken extends IOAuthToken {
	providerId: string;
	scopes: string[];
	storedAt: number;
	userInfo?: {
		sub?: string;
		email?: string;
		name?: string;
		picture?: string;
	};
}

/**
 * Result of checking OAuth tokens for providers
 */
export interface IOAuthTokenCheckResult {
	tokens: Record<string, IOAuthToken>;
	missingProviders: IOAuthProvider[];
}

/**
 * Interface for OAuth token storage implementations.
 * Desktop app uses Dexie, web app might use different storage.
 */
export interface IOAuthTokenStore {
	getToken(providerId: string): Promise<IStoredOAuthToken | undefined>;
	setToken(token: IStoredOAuthToken): Promise<void>;
	deleteToken(providerId: string): Promise<void>;
	getAllTokens(): Promise<IStoredOAuthToken[]>;
	isExpired(token: IStoredOAuthToken, bufferMs?: number): boolean;
}

/**
 * Pending OAuth authorization request
 */
export interface IOAuthPendingAuth {
	state: string;
	providerId: string;
	codeVerifier: string;
	redirectUri: string;
	scopes: string[];
	initiatedAt: number;
	appId?: string;
	boardId?: string;
}

/**
 * Extended token store with pending auth support
 */
export interface IOAuthTokenStoreWithPending extends IOAuthTokenStore {
	setPendingAuth(pending: IOAuthPendingAuth): Promise<void>;
	getPendingAuth(state: string): Promise<IOAuthPendingAuth | undefined>;
	consumePendingAuth(state: string): Promise<IOAuthPendingAuth | undefined>;
	cleanupPendingAuth(): Promise<void>;
}

/**
 * Platform-specific OAuth runtime operations.
 * Abstraction for platform-specific APIs (Tauri, browser, etc.)
 */
export interface IOAuthRuntime {
	/** Open a URL in the system browser or webview */
	openUrl(url: string): Promise<void>;
	/** Make an HTTP POST request */
	httpPost(
		url: string,
		body: string,
		headers?: Record<string, string>,
	): Promise<{
		ok: boolean;
		status: number;
		json: () => Promise<unknown>;
		text: () => Promise<string>;
	}>;
	/** Make an HTTP GET request */
	httpGet(
		url: string,
		headers?: Record<string, string>,
	): Promise<{
		ok: boolean;
		status: number;
		json: () => Promise<unknown>;
		text: () => Promise<string>;
	}>;
}

/**
 * OAuth callback data received from the authorization server
 */
export interface IOAuthCallbackData {
	url: string;
	code: string | null;
	state: string | null;
	id_token: string | null;
	access_token: string | null;
	token_type: string | null;
	expires_in: string | null;
	scope: string | null;
	error: string | null;
	error_description: string | null;
}
