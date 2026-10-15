import {
	renewOidcSession,
	requiresInteractiveSignIn,
} from "@flow-like/flow-like-ui/lib/auth/oidc-renewal";
import type { AuthContextProps } from "react-oidc-context";
import { reportUnauthorized } from "./auth-session";

type User = NonNullable<AuthContextProps["user"]>;
interface SessionState {
	user?: User;
	tokens: Set<string>;
	ended: boolean;
	generation: number;
	renewing?: Promise<User>;
	refusal?: { token?: string; error: unknown };
}
const sessions = new WeakMap<AuthContextProps["signinSilent"], SessionState>();

function session(auth: AuthContextProps): SessionState {
	let state = sessions.get(auth.signinSilent);
	if (!state) {
		state = { tokens: new Set(), ended: false, generation: 0 };
		sessions.set(auth.signinSilent, state);
		const current = state;
		auth.events?.addUserLoaded?.((user) => {
			if (current.user?.profile?.sub !== user.profile?.sub)
				current.generation++;
			current.user = user;
			current.tokens.add(user.access_token);
			current.ended = false;
			current.refusal = undefined;
		});
		auth.events?.addUserUnloaded?.(() => {
			current.user = undefined;
			current.ended = true;
			current.generation++;
			current.refusal = undefined;
		});
	}
	return state;
}

function currentUser(auth: AuthContextProps): User | undefined {
	if (typeof auth.signinSilent !== "function") return auth.user ?? undefined;
	const state = session(auth);
	if (state.ended) return undefined;
	const incoming = auth.user;
	if (incoming && !state.tokens.has(incoming.access_token)) {
		state.tokens.add(incoming.access_token);
		state.user = incoming;
		state.refusal = undefined;
	}
	if (incoming?.profile?.sub !== state.user?.profile?.sub) return undefined;
	return state.user;
}

function expired(user: User): boolean {
	return (
		user.expired === true ||
		(user.expires_at !== undefined && user.expires_at <= Date.now() / 1000 + 30)
	);
}

function refused(message: string): Error & { error: string } {
	return Object.assign(new Error(message), { error: "login_required" });
}

/** Reuse a renewal even before React has published the replacement auth context. */
export function renewWebSession(
	auth: AuthContextProps,
	rejectedToken?: string,
): Promise<User> {
	if (typeof auth.signinSilent !== "function") {
		return Promise.reject(refused("Sign in again to continue."));
	}
	const state = session(auth);
	const user = currentUser(auth);
	if (state.ended || !user)
		return Promise.reject(refused("The session has ended."));
	if (user.access_token !== rejectedToken && !expired(user))
		return Promise.resolve(user);
	if (state.refusal?.token === user.access_token)
		return Promise.reject(state.refusal.error);
	if (state.renewing) return state.renewing;
	const generation = state.generation;
	state.renewing = renewOidcSession(auth)
		.then((renewed) => {
			if (state.ended || generation !== state.generation)
				throw new Error("The account changed during session renewal.");
			if (
				!renewed?.access_token ||
				renewed.expired ||
				renewed.profile?.sub !== user.profile?.sub
			) {
				throw refused("Sign in again to continue.");
			}
			if (rejectedToken && renewed.access_token === rejectedToken) {
				throw refused(
					"The renewed session was rejected. Sign in again to continue.",
				);
			}
			state.user = renewed;
			state.tokens.add(renewed.access_token);
			state.refusal = undefined;
			return renewed;
		})
		.catch((error: unknown) => {
			if (requiresInteractiveSignIn(error))
				state.refusal = { token: user.access_token, error };
			throw error;
		})
		.finally(() => {
			state.renewing = undefined;
		});
	return state.renewing;
}

/** Refresh before expiry and retry an authenticated 401 once with the returned token. */
export async function fetchWithSession(
	url: string,
	init: RequestInit,
	auth?: AuthContextProps,
): Promise<Response> {
	const headers = new Headers(init.headers);
	const suppliedToken = headers.get("Authorization");
	const accountToken = auth?.user?.access_token;
	if (!auth || (suppliedToken && suppliedToken !== `Bearer ${accountToken}`)) {
		return fetch(url, init);
	}
	let user = currentUser(auth);
	if (user && expired(user)) {
		try {
			user = await renewWebSession(auth, user.access_token);
		} catch (error) {
			if (requiresInteractiveSignIn(error))
				reportUnauthorized(auth, accountToken);
			throw error;
		}
	}
	if (user?.access_token)
		headers.set("Authorization", `Bearer ${user.access_token}`);
	else headers.delete("Authorization");
	const usedToken = user?.access_token;
	const response = await fetch(url, { ...init, headers });
	if (response.status !== 401) return response;
	if (!usedToken) {
		reportUnauthorized(auth, usedToken);
		return response;
	}
	try {
		user = await renewWebSession(auth, usedToken);
	} catch (error) {
		if (requiresInteractiveSignIn(error)) reportUnauthorized(auth, usedToken);
		if (!requiresInteractiveSignIn(error)) throw error;
		return response;
	}
	// A caller-owned stream cannot be sent twice. Its next request uses the fresh session.
	if (
		typeof ReadableStream !== "undefined" &&
		init.body instanceof ReadableStream
	)
		return response;
	await response.body?.cancel();
	headers.set("Authorization", `Bearer ${user.access_token}`);
	const retried = await fetch(url, { ...init, headers });
	if (retried.status === 401) reportUnauthorized(auth, user.access_token);
	return retried;
}
