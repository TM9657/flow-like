import { createOAuthService } from "@tm9657/flow-like-ui";
import { oauthTokenStore } from "./oauth-db";
import { tauriOAuthRuntime } from "./tauri-oauth-runtime";

export type { IOAuthProvider } from "@tm9657/flow-like-ui";

export const oauthService = createOAuthService({
	runtime: tauriOAuthRuntime,
	tokenStore: oauthTokenStore,
});
