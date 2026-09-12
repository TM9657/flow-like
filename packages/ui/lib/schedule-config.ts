import type { IEvent } from "./schema/flow/event";
import { parseUint8ArrayToJson } from "./uint8";

/** Event type recorded for schedule-triggered events. */
export const CRON_EVENT_TYPE = "cron";

/**
 * The schedule half of an event's configuration, and nothing else.
 *
 * An event config is one JSON blob that also carries transport secrets — an
 * `auth_token` for HTTP-triggered events — so anything handing a config to a
 * schedule view projects it down to these fields first. The account-scoped
 * schedules endpoint does exactly this server-side; this mirrors it for events
 * read from a local store, so both halves of a merged list have one shape.
 */
export interface IScheduleConfig {
	expression?: string | null;
	timezone?: string | null;
	scheduled_for?: { date?: string; time?: string } | null;
	last_fired?: string | null;
}

function firstString(
	config: Record<string, unknown>,
	keys: readonly string[],
): string | undefined {
	for (const key of keys) {
		const value = config[key];
		if (typeof value === "string" && value) return value;
	}
	return undefined;
}

/** Keep the aliases the sink layer accepts, but produce one spelling. */
export function projectScheduleConfig(
	config: Record<string, unknown> | undefined | null,
): IScheduleConfig {
	if (!config) return {};
	const scheduled = config.scheduled_for ?? config.scheduledFor;
	const local =
		scheduled && typeof scheduled === "object"
			? (scheduled as { date?: unknown; time?: unknown })
			: undefined;
	return {
		expression: firstString(config, [
			"expression",
			"cron_expression",
			"cronExpression",
			"cron",
			"schedule",
		]),
		timezone: firstString(config, [
			"timezone",
			"tz",
			"cron_timezone",
			"cronTimezone",
		]),
		scheduled_for:
			typeof local?.date === "string" && typeof local?.time === "string"
				? { date: local.date, time: local.time }
				: undefined,
		last_fired: firstString(config, ["last_fired", "lastFired"]),
	};
}

/** The schedule config of an active cron event, or null for anything else. */
export function scheduleConfigFromEvent(event: IEvent): IScheduleConfig | null {
	if (!event.active || event.event_type !== CRON_EVENT_TYPE) return null;
	try {
		return projectScheduleConfig(parseUint8ArrayToJson(event.config));
	} catch {
		return null;
	}
}
