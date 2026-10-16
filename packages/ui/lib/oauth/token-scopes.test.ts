import { describe, expect, it } from "vitest";
import { getMissingOAuthScopes, hasRequiredScopes } from "./token-scopes";

const identityScopes = ["openid", "email", "profile"];
const googleIdentityScopes = [
	"openid",
	"https://www.googleapis.com/auth/userinfo.email",
	"https://www.googleapis.com/auth/userinfo.profile",
];
const calendarScope = "https://www.googleapis.com/auth/calendar.events";

describe("OAuth scope matching", () => {
	it.each([
		[googleIdentityScopes, identityScopes],
		[identityScopes, googleIdentityScopes],
	])(
		"accepts Google's identity aliases in either direction",
		(scopes, required) => {
			expect(
				hasRequiredScopes({ providerId: "google", scopes }, required),
			).toBe(true);
		},
	);

	it("still requires every API permission without changing the stored grant", () => {
		const grant = { providerId: "google", scopes: [...googleIdentityScopes] };
		expect(
			getMissingOAuthScopes(grant, [...identityScopes, calendarScope]),
		).toEqual([calendarScope]);
		expect(grant.scopes).toEqual(googleIdentityScopes);
		expect(hasRequiredScopes(grant, [...identityScopes, calendarScope])).toBe(
			false,
		);
	});

	it("does not apply Google aliases to other providers", () => {
		expect(
			getMissingOAuthScopes(
				{ providerId: "other", scopes: googleIdentityScopes },
				identityScopes,
			),
		).toEqual(["email", "profile"]);
	});

	it("keeps permissions case-sensitive and does not infer broader API grants", () => {
		const grant = {
			providerId: "google",
			scopes: ["Email", "https://www.googleapis.com/auth/calendar"],
		};
		expect(getMissingOAuthScopes(grant, ["email", calendarScope])).toEqual([
			"email",
			calendarScope,
		]);
	});

	it("does not treat an empty grant as authorization", () => {
		const grant = { providerId: "google", scopes: [] };
		expect(hasRequiredScopes(grant, identityScopes)).toBe(false);
		expect(hasRequiredScopes(grant, [])).toBe(true);
	});
});
