import {
	BUILTIN_RUNTIME_EVENT_TYPE_SET,
	deriveRouteMappings,
	isUsableRuntimeEvent,
	resolveRouteMapping,
} from "../../../../lib/runtime-route";
import type { IEvent } from "../../../../lib/schema/flow/event";
import { parseHomeEmbedTarget } from "../../../home/home-content/config";
import { issue, stringArg } from "./shared";
import type { HomeToolIssue } from "./types";

/** Check the exact Event the runtime resolves, including root and default targets. */
export function validateHomeEmbedReference(
	config: Record<string, unknown>,
	events: readonly IEvent[],
	path: string,
): HomeToolIssue[] {
	const appId = stringArg(config, "appId");
	const target = parseHomeEmbedTarget(config);
	if (target.eventId) {
		const event = events.find((entry) => entry.id === target.eventId);
		if (!event) {
			return [
				issue(
					"error",
					"home_app_event_missing",
					`${path}.eventId`,
					`Event '${target.eventId}' is not available in app '${appId}'.`,
				),
			];
		}
		if (!event.active) {
			return [
				issue(
					"error",
					"home_app_event_inactive",
					`${path}.eventId`,
					`Event '${event.id}' is inactive.`,
				),
			];
		}
		return isUsableRuntimeEvent(event, BUILTIN_RUNTIME_EVENT_TYPE_SET)
			? []
			: [
					issue(
						"error",
						"home_app_event_not_embeddable",
						`${path}.eventId`,
						`Event '${event.id}' does not expose a Home-compatible page, chat, form, or quick action.`,
					),
				];
	}
	const { mapping, missed } = resolveRouteMapping(
		deriveRouteMappings(events),
		target.routePath,
	);
	const event = mapping
		? events.find((entry) => entry.id === mapping.eventId)
		: undefined;
	if (!missed && isUsableRuntimeEvent(event, BUILTIN_RUNTIME_EVENT_TYPE_SET))
		return [];
	const targetPath = `${path}.${config.target === "route" ? "route" : "target"}`;
	if (
		!mapping &&
		!missed &&
		events.some((entry) =>
			isUsableRuntimeEvent(entry, BUILTIN_RUNTIME_EVENT_TYPE_SET),
		)
	) {
		return [
			issue(
				"warning",
				"home_app_landing_fallback",
				targetPath,
				"No default route is configured. Home will use an available interface, which can depend on the last-used Event.",
			),
		];
	}
	return [
		issue(
			"error",
			"home_app_route_missing",
			targetPath,
			`Route '${target.routePath}' does not resolve to an active Home-compatible interface in app '${appId}'.`,
		),
	];
}
