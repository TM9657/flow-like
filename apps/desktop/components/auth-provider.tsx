"use client";
import { listen } from "@tauri-apps/api/event";
import { getAllWindows } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useBackend, useInvoke } from "@tm9657/flow-like-ui";
import { Amplify } from "aws-amplify";
import {
	type AuthTokens,
	type TokenProvider,
	decodeJWT,
} from "aws-amplify/auth";
import {
	type INavigator,
	type IWindow,
	type NavigateParams,
	UserManager,
	type UserManagerSettings,
	WebStorageStateStore,
} from "oidc-client-ts";
import { useEffect, useState } from "react";
import { AuthProvider, useAuth } from "react-oidc-context";
import { get } from "../lib/api";
import { ProfileSyncer, TauriBackend } from "./tauri-provider";

export class OIDCTokenProvider implements TokenProvider {
	constructor(private readonly userManager: UserManager) {}
	async getTokens(options?: {
		forceRefresh?: boolean;
	}): Promise<AuthTokens | null> {
		console.warn("Getting tokens from OIDCTokenProvider...");
		const user = await this.userManager.getUser();
		if (!user?.access_token || !user?.id_token) {
			return null;
		}

		const accessToken = decodeJWT(user.access_token);
		const idToken = decodeJWT(user.id_token);

		return {
			accessToken: accessToken,
			idToken: idToken,
		};
	}
}

class TauriWindow implements IWindow {
	private abort: ((reason: Error) => void) | undefined;
	close() {
		return;
	}
	async navigate(params: NavigateParams): Promise<never> {
		openUrl(params.url);

		const promise = new Promise((resolve, reject) => {
			this.abort = reject;
		});

		return promise as Promise<never>;
	}
}

class TauriRedirectNavigator implements INavigator {
	async prepare(params: unknown): Promise<IWindow> {
		return new TauriWindow();
	}

	async callback(url: string, params?: unknown): Promise<void> {
		return;
	}
}

export function DesktopAuthProvider({
	children,
}: Readonly<{ children: React.ReactNode }>) {
	const [openIdAuthConfig, setOpenIdAuthConfig] =
		useState<UserManagerSettings>();
	const [userManager, setUserManager] = useState<UserManager>();
	const backend = useBackend();
	const currentProfile = useInvoke(
		backend.userState.getProfile,
		backend.userState,
		[],
	);

	useEffect(() => {
		if (!currentProfile.data) {
			console.warn("[DESKTOPAUTH] No profile data available yet.");
			return;
		}
		(async () => {
			console.log("[DESKTOPAUTH] Fetching OpenID configuration!!!");
			const response = await get<any>(currentProfile.data, "auth/openid");
			if (response) {
				if (process.env.NEXT_PUBLIC_REDIRECT_URL)
					response.redirect_uri = process.env.NEXT_PUBLIC_REDIRECT_URL;
				if (process.env.NEXT_PUBLIC_REDIRECT_LOGOUT_URL)
					response.post_logout_redirect_uri =
						process.env.NEXT_PUBLIC_REDIRECT_LOGOUT_URL;
				const store = new WebStorageStateStore({
					store: localStorage,
				});
				response.userStore = store;
				response.automaticSilentRenew = true;
				const navigator = new TauriRedirectNavigator();
				const userManagerInstance = new UserManager(response, navigator);
				response.userManager = userManagerInstance;
				const tokenProvider = new OIDCTokenProvider(userManagerInstance);
				if (response.cognito)
					Amplify.configure(
						{
							Auth: {
								Cognito: {
									userPoolClientId: response.client_id,
									userPoolId: response.cognito.user_pool_id,
								},
							},
						},
						{
							Auth: {
								tokenProvider: tokenProvider,
							},
						},
					);
				setUserManager(userManagerInstance);
				setOpenIdAuthConfig(response);
			}
		})();
	}, [currentProfile.data]);

	useEffect(() => {
		if (!openIdAuthConfig) return;

		async function debugListener(event: Event) {
			const url = (event as CustomEvent).detail.url;
			console.log("Debug OIDC URL:", url);
			await userManager?.signinRedirectCallback(url);
			const windows = await getAllWindows();
			for (const window of windows) {
				if (window.label === "oidcFlow") {
					window.close();
				}
			}
		}

		const globalListen = window.addEventListener("debug-oidc", debugListener);

		const unlisten = listen<{ url: string }>("oidc/url", async (event) => {
			const rawUrl = event.payload.url;

			const normalizeTo = (target: string) => {
				try {
					const targetUrl = new URL(target);
					const sourceUrl = new URL(rawUrl);
					targetUrl.search = sourceUrl.search;
					targetUrl.hash = sourceUrl.hash;
					return targetUrl.toString();
				} catch {
					return rawUrl;
				}
			};

			const isDeepLink = rawUrl.startsWith("flow-like://");
			const signinUrl = isDeepLink
				? normalizeTo(openIdAuthConfig.redirect_uri)
				: rawUrl;
			const logoutUrl =
				isDeepLink && openIdAuthConfig.post_logout_redirect_uri
					? normalizeTo(openIdAuthConfig.post_logout_redirect_uri)
					: rawUrl;

			if (signinUrl.startsWith(openIdAuthConfig.redirect_uri)) {
				await userManager?.signinRedirectCallback(signinUrl);
				const windows = await getAllWindows();
				for (const window of windows) {
					if (window.label === "oidcFlow") {
						window.close();
					}
				}
			}

			if (
				openIdAuthConfig.post_logout_redirect_uri &&
				logoutUrl.startsWith(openIdAuthConfig.post_logout_redirect_uri)
			) {
				const windows = await getAllWindows();
				for (const window of windows) {
					if (window.label === "oidcFlow") {
						window.close();
					}
				}
			}

			if (signinUrl.includes("/login?id_token_hint=")) {
				const windows = await getAllWindows();
				for (const window of windows) {
					if (window.label === "oidcFlow") {
						window.close();
					}
				}
			}
		});

		return () => {
			unlisten.then((unsub) => unsub());
			window.removeEventListener("debug-oidc", debugListener);
		};
	}, [userManager, openIdAuthConfig]);

	if (!openIdAuthConfig)
		return <AuthProvider key="loading-auth-config">{children}</AuthProvider>;

	return (
		<AuthProvider
			key={openIdAuthConfig.client_id}
			{...openIdAuthConfig}
			automaticSilentRenew={true}
			userStore={
				new WebStorageStateStore({
					store: localStorage,
				})
			}
		>
			<AuthInner>{children}</AuthInner>
		</AuthProvider>
	);
}

function AuthInner({ children }: Readonly<{ children: React.ReactNode }>) {
	const auth = useAuth();
	const backend = useBackend();

	useEffect(() => {
		if (!auth) return;
		if (!auth.isAuthenticated) {
			return;
		}

		if (!auth.user?.id_token) {
			console.warn("User is authenticated but no ID token found.");
			return;
		}

		if (backend instanceof TauriBackend) {
			console.log("Pushing auth context to backend:", auth);
			backend.pushAuthContext(auth);
		}
	}, [auth?.isAuthenticated, auth?.user?.id_token, backend]);

	useEffect(() => {
		if (!auth) return;

		(async () => {
			try {
				const existingUser = auth.user;

				if (existingUser && !existingUser.expired) {
					return;
				}

				if (existingUser?.expired) {
					try {
						const user = await auth?.signinSilent();
						if (!user) {
							console.warn(
								"Silent login returned no user, attempting redirect login.",
							);
							await auth?.signinRedirect();
						}
					} catch (silentError) {
						console.warn(
							"Silent login failed, attempting normal login:",
							silentError,
						);

						try {
							await auth?.signinRedirect();
						} catch (redirectError) {
							console.error(
								"Both silent and redirect login failed:",
								redirectError,
							);
						}
					}
				}
			} catch (error) {
				console.error("Login process failed:", error);
			}
		})();
	}, [auth.user?.profile?.sub]);

	return <>
		<ProfileSyncer auth={{ isAuthenticated: auth.isAuthenticated, accessToken: auth.user?.access_token }} />
		{children}
	</>;
}
