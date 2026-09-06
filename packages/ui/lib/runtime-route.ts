import type { IRouteMapping } from "../state/backend-state/route-state";
import { normalizeRoutePath, routePathsEqual } from "./route-path";
import type { IEvent } from "./schema/flow/event";

/** Built-in interfaces available to both the app runtime and Home embeds. */
export const BUILTIN_RUNTIME_EVENT_TYPES = {
	chat: "simple_chat",
	form: "generic_form",
	quickAction: "quick_action",
} as const;

export const BUILTIN_RUNTIME_EVENT_TYPE_SET: ReadonlySet<string> = new Set(
	Object.values(BUILTIN_RUNTIME_EVENT_TYPES),
);

export interface IRouteResolution {
	readonly mapping: IRouteMapping | null;
	/** A non-root route was missing. The mapping is the default fallback, if any. */
	readonly missed: boolean;
}

/** Resolve canonical paths and report a miss even when a default route exists. */
export function resolveRouteMapping(
	availableRoutes: readonly IRouteMapping[],
	routePath: string | null | undefined,
): IRouteResolution {
	const defaultRoute =
		availableRoutes.find((route) => routePathsEqual(route.path, "/")) ?? null;
	const requested = normalizeRoutePath(routePath);
	if (requested === "/") return { mapping: defaultRoute, missed: false };
	const matched =
		availableRoutes.find((route) => routePathsEqual(route.path, requested)) ??
		null;
	return matched
		? { mapping: matched, missed: false }
		: { mapping: defaultRoute, missed: true };
}

/** First canonical path wins; explicit routes precede synthesized default routes. */
export function deriveRouteMappings(
	events: readonly IEvent[] | null | undefined,
): IRouteMapping[] {
	const mappings = new Map<string, IRouteMapping>();
	const add = (path: string, eventId: string) => {
		const key = normalizeRoutePath(path);
		if (!mappings.has(key)) mappings.set(key, { path, eventId });
	};
	for (const event of events ?? []) {
		const path = event.route?.trim();
		if (path) add(path, event.id);
	}
	for (const event of events ?? []) {
		if (!event.is_default || event.route?.trim()) continue;
		add("/", event.id);
	}
	return [...mappings.values()];
}

/** An inactive or headless Event is configuration data, not an interface target. */
export function isUsableRuntimeEvent(
	event: IEvent | null | undefined,
	usableEventTypes: { has(value: string): boolean },
): boolean {
	return Boolean(
		event?.active &&
			(event.default_page_id?.trim() || usableEventTypes.has(event.event_type)),
	);
}
