import type { IStoredOAuthToken } from "./types";

function normalizeScope(providerId: string, scope: string): string {
	// Google returns these URLs for the requested OpenID email/profile scopes.
	if (providerId === "google") {
		if (scope === "https://www.googleapis.com/auth/userinfo.email")
			return "email";
		if (scope === "https://www.googleapis.com/auth/userinfo.profile")
			return "profile";
	}
	return scope;
}

export function getMissingOAuthScopes(
	grant: Pick<IStoredOAuthToken, "providerId" | "scopes">,
	requiredScopes: string[],
): string[] {
	const grantedScopes = new Set(
		(grant.scopes ?? []).map((scope) =>
			normalizeScope(grant.providerId, scope),
		),
	);
	return (requiredScopes ?? []).filter(
		(scope) => !grantedScopes.has(normalizeScope(grant.providerId, scope)),
	);
}

export function hasRequiredScopes(
	grant: Pick<IStoredOAuthToken, "providerId" | "scopes">,
	requiredScopes: string[],
): boolean {
	return getMissingOAuthScopes(grant, requiredScopes).length === 0;
}
