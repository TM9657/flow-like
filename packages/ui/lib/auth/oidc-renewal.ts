interface RenewalUser {
	access_token: string;
	refresh_token?: string;
	expired?: boolean;
	profile?: { sub?: string };
}

interface RenewalManager<User extends RenewalUser, Args extends unknown[]> {
	settings: { authority: string; client_id: string };
	events?: object;
	getUser(raiseEvent?: boolean): Promise<User | null>;
	signinSilent(...args: Args): Promise<User | null>;
	signinRedirectCallback?(url?: string): Promise<User>;
	removeUser(): Promise<void>;
}

const coordinated = new WeakSet<object>();
const fallbackLocks = new Map<string, Promise<unknown>>();
const renewers = new WeakMap<object, () => Promise<RenewalUser | null>>();

/** Timer and API recovery share one refresh; Web Locks extend this across tabs. */
export function coordinateOidcRenewal<
	User extends RenewalUser,
	Args extends unknown[],
>(manager: RenewalManager<User, Args>): void {
	if (coordinated.has(manager)) return;
	coordinated.add(manager);
	const signinSilent = manager.signinSilent.bind(manager);
	const removeUser = manager.removeUser.bind(manager);
	const lockName = `flow-like:oidc:${JSON.stringify([
		manager.settings.authority,
		manager.settings.client_id,
	])}`;
	let pending: Promise<User | null> | undefined;
	let generation = 0;
	const locked = async <T>(operation: () => Promise<T>): Promise<T> => {
		if (typeof navigator !== "undefined" && navigator.locks?.request) {
			return await navigator.locks.request(lockName, operation);
		}
		const previous = fallbackLocks.get(lockName) ?? Promise.resolve();
		const next = previous.catch(() => undefined).then(operation);
		fallbackLocks.set(lockName, next);
		void next
			.finally(() => {
				if (fallbackLocks.get(lockName) === next)
					fallbackLocks.delete(lockName);
			})
			.catch(() => undefined);
		return next;
	};
	manager.removeUser = async () => {
		generation++;
		await locked(removeUser);
	};
	if (manager.signinRedirectCallback) {
		const completeSignIn = manager.signinRedirectCallback.bind(manager);
		manager.signinRedirectCallback = (url) =>
			locked(async () => {
				const user = await completeSignIn(url);
				generation++;
				return user;
			});
	}
	manager.signinSilent = (...args) => {
		if (pending) return pending;
		const started = generation;
		pending = (async () => {
			const before = await manager.getUser();
			return locked(async () => {
				const current = await manager.getUser();
				if (started !== generation || (before && !current)) {
					throw new Error("The session ended before it could be renewed.");
				}
				if (before && current && before.profile?.sub !== current.profile?.sub) {
					throw new Error(
						"The account changed before the session could be renewed.",
					);
				}
				if (
					current &&
					!current.expired &&
					before &&
					(current.access_token !== before.access_token ||
						current.refresh_token !== before.refresh_token)
				) {
					// Another tab already rotated this grant. Load its user into this context.
					return manager.getUser(true);
				}
				const renewed = await signinSilent(...args);
				if (started !== generation) {
					await removeUser();
					throw new Error("The session ended while it was being renewed.");
				}
				if (before && renewed && before.profile?.sub !== renewed.profile?.sub) {
					throw new Error(
						"The account changed while the session was being renewed.",
					);
				}
				return renewed;
			});
		})().finally(() => {
			pending = undefined;
		});
		return pending;
	};
	if (manager.events) {
		renewers.set(manager.events, () =>
			manager.signinSilent(...([] as unknown as Args)),
		);
	}
}

/** react-oidc-context converts renewal errors to null; recovery needs the original error. */
export function renewOidcSession<User extends RenewalUser>(auth: {
	events?: object;
	signinSilent(): Promise<User | null>;
}): Promise<User | null> {
	const renew = auth.events && renewers.get(auth.events);
	return renew ? (renew() as Promise<User | null>) : auth.signinSilent();
}

/** Only an explicit identity-provider refusal means the saved grant cannot recover. */
export function requiresInteractiveSignIn(error: unknown): boolean {
	const code = (error as { error?: unknown } | null)?.error;
	return (
		typeof code === "string" &&
		[
			"invalid_grant",
			"login_required",
			"consent_required",
			"interaction_required",
			"account_selection_required",
		].includes(code)
	);
}
