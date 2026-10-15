import "fake-indexeddb/auto";
import Dexie from "dexie";
import { expect, it } from "vitest";

it("drops ownerless legacy tokens and consents instead of attributing them to the next login", async () => {
	const legacy = new Dexie("OAuthTokens");
	legacy.version(2).stores({
		tokens: "providerId, expiresAt",
		pendingAuth: "state, providerId, initiatedAt",
		consents: "id, appId, providerId, consentedAt",
	});
	await legacy
		.table("tokens")
		.put({ providerId: "example", access_token: "another-users-token" });
	await legacy.table("consents").put({
		id: "app:example",
		appId: "app",
		providerId: "example",
		scopes: ["write"],
	});
	legacy.close();
	const { oauthDB, configureOAuthSession, oauthTokenStore, oauthConsentStore } =
		await import("./oauth-db");
	configureOAuthSession(() => ({
		hub: "https://hub.test",
		subject: "next-user",
		issuer: "https://issuer.test",
	}));
	expect(await oauthTokenStore.getAllTokens()).toEqual([]);
	expect(await oauthConsentStore.getAllConsents()).toEqual([]);
	expect(oauthDB.tables.map((table) => table.name)).not.toContain("tokens");
	await oauthDB.delete();
});
