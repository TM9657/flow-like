type NavigationGuard = () => boolean;
const guards = new Map<string, Set<NavigationGuard>>();

function scopeKey(appId: string | undefined, surfaceId: string | undefined) {
	return JSON.stringify([appId, surfaceId]);
}

/** Keep document drafts mounted when an A2UI action tries to leave their surface. */
export function registerA2UINavigationGuard(
	appId: string | undefined,
	surfaceId: string | undefined,
	guard: NavigationGuard,
) {
	const key = scopeKey(appId, surfaceId);
	const scoped = guards.get(key) ?? new Set<NavigationGuard>();
	scoped.add(guard);
	guards.set(key, scoped);
	return () => {
		scoped.delete(guard);
		if (!scoped.size) guards.delete(key);
	};
}

export function canLeaveA2UISurface(
	appId: string | undefined,
	surfaceId: string | undefined,
): boolean {
	for (const guard of guards.get(scopeKey(appId, surfaceId)) ?? []) {
		if (!guard()) return false;
	}
	return true;
}
