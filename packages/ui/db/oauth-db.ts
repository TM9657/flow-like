import Dexie, { type EntityTable } from "dexie";
import type {
	IOAuthPendingAuth,
	IOAuthTokenStoreWithPending,
	IStoredOAuthToken,
} from "../lib/oauth/types";

interface IDexieOAuthToken {
	providerId: string;
	accessToken: string;
	refreshToken?: string;
	expiresAt?: number;
	tokenType?: string;
	scopes: string[];
	storedAt: number;
	userInfo?: {
		sub?: string;
		email?: string;
		name?: string;
		picture?: string;
	};
}

interface IDexieOAuthConsent {
	/** Composite key: appId:providerId */
	id: string;
	appId: string;
	providerId: string;
	consentedAt: number;
	scopes: string[];
}

const oauthDB = new Dexie("OAuthTokens") as Dexie & {
	tokens: EntityTable<IDexieOAuthToken, "providerId">;
	pendingAuth: EntityTable<IOAuthPendingAuth, "state">;
	consents: EntityTable<IDexieOAuthConsent, "providerId">;
};

oauthDB.version(1).stores({
	tokens: "providerId, expiresAt",
	pendingAuth: "state, providerId, initiatedAt",
});

oauthDB.version(2).stores({
	tokens: "providerId, expiresAt",
	pendingAuth: "state, providerId, initiatedAt",
	consents: "id, appId, providerId, consentedAt",
});

export { oauthDB };

function fromDexieFormat(token: IDexieOAuthToken): IStoredOAuthToken {
	return {
		providerId: token.providerId,
		access_token: token.accessToken,
		refresh_token: token.refreshToken,
		expires_at: token.expiresAt,
		token_type: token.tokenType,
		scopes: token.scopes,
		storedAt: token.storedAt,
		userInfo: token.userInfo,
	};
}

function toDexieFormat(token: IStoredOAuthToken): IDexieOAuthToken {
	return {
		providerId: token.providerId,
		accessToken: token.access_token,
		refreshToken: token.refresh_token,
		expiresAt: token.expires_at,
		tokenType: token.token_type,
		scopes: token.scopes,
		storedAt: token.storedAt,
		userInfo: token.userInfo,
	};
}

export interface IOAuthConsentStore {
	/** Check if user has consented to a provider for a specific app/workflow */
	hasConsent(appId: string, providerId: string): Promise<boolean>;
	/** Check if user has consented to a provider with all required scopes */
	hasConsentWithScopes(
		appId: string,
		providerId: string,
		requiredScopes: string[],
	): Promise<boolean>;
	/** Save consent for a provider in a specific app/workflow */
	setConsent(
		appId: string,
		providerId: string,
		scopes: string[],
	): Promise<void>;
	/** Revoke consent for a provider in a specific app/workflow */
	revokeConsent(appId: string, providerId: string): Promise<void>;
	/** Revoke all consents for an app/workflow */
	revokeAllConsentsForApp(appId: string): Promise<void>;
	/** Get all consents */
	getAllConsents(): Promise<
		{
			appId: string;
			providerId: string;
			consentedAt: number;
			scopes: string[];
		}[]
	>;
	/** Get consented provider IDs for a specific app/workflow */
	getConsentedProviderIds(appId: string): Promise<Set<string>>;
	/** Get consented provider IDs that have all required scopes */
	getConsentedProviderIdsWithScopes(
		appId: string,
		requiredScopesMap: Map<string, string[]>,
	): Promise<Set<string>>;
}

export const oauthConsentStore: IOAuthConsentStore = {
	async hasConsent(appId: string, providerId: string): Promise<boolean> {
		const id = `${appId}:${providerId}`;
		const consent = await oauthDB.consents.get(id);
		return consent !== undefined;
	},

	async setConsent(
		appId: string,
		providerId: string,
		scopes: string[],
	): Promise<void> {
		const id = `${appId}:${providerId}`;
		await oauthDB.consents.put({
			id,
			appId,
			providerId,
			consentedAt: Date.now(),
			scopes,
		});
	},

	async revokeConsent(appId: string, providerId: string): Promise<void> {
		const id = `${appId}:${providerId}`;
		await oauthDB.consents.delete(id);
	},

	async revokeAllConsentsForApp(appId: string): Promise<void> {
		await oauthDB.consents.where("appId").equals(appId).delete();
	},

	async getAllConsents(): Promise<
		{
			appId: string;
			providerId: string;
			consentedAt: number;
			scopes: string[];
		}[]
	> {
		return await oauthDB.consents.toArray();
	},

	async getConsentedProviderIds(appId: string): Promise<Set<string>> {
		const consents = await oauthDB.consents
			.where("appId")
			.equals(appId)
			.toArray();
		return new Set(consents.map((c) => c.providerId));
	},

	async hasConsentWithScopes(
		appId: string,
		providerId: string,
		requiredScopes: string[],
	): Promise<boolean> {
		const id = `${appId}:${providerId}`;
		const consent = await oauthDB.consents.get(id);
		if (!consent) return false;

		// Check if all required scopes are in the consented scopes
		const consentedScopes = new Set(consent.scopes ?? []);
		return requiredScopes.every((scope) => consentedScopes.has(scope));
	},

	async getConsentedProviderIdsWithScopes(
		appId: string,
		requiredScopesMap: Map<string, string[]>,
	): Promise<Set<string>> {
		const consents = await oauthDB.consents
			.where("appId")
			.equals(appId)
			.toArray();

		const validProviders = new Set<string>();
		for (const consent of consents) {
			const requiredScopes = requiredScopesMap.get(consent.providerId);
			if (!requiredScopes || requiredScopes.length === 0) {
				// No specific scopes required, consent is valid
				validProviders.add(consent.providerId);
				continue;
			}

			const consentedScopes = new Set(consent.scopes ?? []);
			const hasAllScopes = requiredScopes.every((scope) =>
				consentedScopes.has(scope),
			);
			if (hasAllScopes) {
				validProviders.add(consent.providerId);
			} else {
				console.log(
					`[OAuth] Consent for ${consent.providerId} is missing scopes. Required:`,
					requiredScopes,
					"Consented:",
					consent.scopes,
				);
			}
		}

		return validProviders;
	},
};

export const oauthTokenStore: IOAuthTokenStoreWithPending = {
	async getToken(providerId: string): Promise<IStoredOAuthToken | undefined> {
		const token = await oauthDB.tokens.get(providerId);
		return token ? fromDexieFormat(token) : undefined;
	},

	async setToken(token: IStoredOAuthToken): Promise<void> {
		await oauthDB.tokens.put(toDexieFormat(token));
	},

	async deleteToken(providerId: string): Promise<void> {
		await oauthDB.tokens.delete(providerId);
	},

	async getAllTokens(): Promise<IStoredOAuthToken[]> {
		const tokens = await oauthDB.tokens.toArray();
		return tokens.map(fromDexieFormat);
	},

	isExpired(token: IStoredOAuthToken, bufferMs = 60000): boolean {
		if (!token.expires_at) return false;
		return Date.now() + bufferMs >= token.expires_at;
	},

	async setPendingAuth(pending: IOAuthPendingAuth): Promise<void> {
		await oauthDB.pendingAuth.put(pending);
	},

	async getPendingAuth(state: string): Promise<IOAuthPendingAuth | undefined> {
		return await oauthDB.pendingAuth.get(state);
	},

	async consumePendingAuth(
		state: string,
	): Promise<IOAuthPendingAuth | undefined> {
		const pending = await oauthDB.pendingAuth.get(state);
		if (pending) {
			await oauthDB.pendingAuth.delete(state);
		}
		return pending;
	},

	async cleanupPendingAuth(): Promise<void> {
		const tenMinutesAgo = Date.now() - 10 * 60 * 1000;
		await oauthDB.pendingAuth
			.where("initiatedAt")
			.below(tenMinutesAgo)
			.delete();
	},
};
