import type { IProfile } from "../types";
import { getApiOrigin } from "./api-url";

/** Shared invitations start at the API's public preview and app handoff page. */
export function getInviteLink(
	appId: string,
	token: string,
	profile?: Partial<IProfile> | null,
): string {
	const url = new URL(getApiOrigin(profile));
	if (
		!/^https?:$/.test(url.protocol) ||
		url.username ||
		url.password ||
		url.search ||
		url.hash
	) {
		throw new Error("The invitation API URL must be an HTTP(S) base URL");
	}
	// A deployment may provide its API origin or its versioned API base.
	url.pathname = `${url.pathname.replace(/\/+$/, "").replace(/\/api\/v1$/, "")}/join`;
	url.search = new URLSearchParams({ appId, token }).toString();
	return url.href;
}

export function getInviteDeepLink(appId: string, token: string): string {
	return `flow-like://join?${new URLSearchParams({ appId, token })}`;
}
