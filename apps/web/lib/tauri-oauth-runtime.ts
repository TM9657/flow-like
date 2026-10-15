import {
	type IOAuthBinding,
	type IOAuthRuntime,
	normalizeOAuthHub,
	oauthSessionKey,
	useBackendStore,
} from "@flow-like/flow-like-ui";
import type { AuthContextProps } from "react-oidc-context";
import { fetchWithSession } from "./session-request";

async function oauthRequest(
	url: string,
	init: RequestInit,
	binding?: IOAuthBinding,
): Promise<Response> {
	if (!new URL(url).pathname.includes("/api/v1/oauth/"))
		return fetch(url, init);
	const backend = useBackendStore.getState().backend as {
		profile?: { hub?: string };
		auth?: AuthContextProps;
	} | null;
	const hub = backend?.profile?.hub;
	if (
		!hub ||
		!url.startsWith(`${normalizeOAuthHub(hub)}/api/v1/oauth/`) ||
		!backend.auth?.user?.access_token ||
		!backend.auth.user.profile.sub ||
		(binding &&
			binding.sessionKey !==
				oauthSessionKey({
					hub,
					subject: backend.auth.user?.profile.sub ?? "",
					issuer: backend.auth.user?.profile.iss,
				}))
	)
		throw new Error("Sign in to the OAuth connection's Hub before continuing.");
	return fetchWithSession(url, init, backend.auth);
}

export const webOAuthRuntime: IOAuthRuntime = {
	async openUrl(url) {
		window.open(url, "_blank", "noopener,noreferrer");
	},
	async httpPost(url, body, headers, binding) {
		return oauthRequest(url, { method: "POST", body, headers }, binding);
	},
	async httpGet(url, headers, binding) {
		return oauthRequest(url, { method: "GET", headers }, binding);
	},
};

export const tauriOAuthRuntime = webOAuthRuntime;
