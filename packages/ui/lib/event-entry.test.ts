import { describe, expect, test } from "bun:test";
import { describeEventEntry } from "./event-entry";
import type { IEvent } from "./schema/flow/event";

const event = { event_type: "cron" } as IEvent;

describe("cron event entries", () => {
	test("shows the date, time and timezone for a one-time schedule", () => {
		expect(
			describeEventEntry(event, {
				expression: null,
				scheduled_for: { date: "2028-02-29", time: "09:30" },
				timezone: "Europe/Berlin",
			}),
		).toEqual({
			text: "2028-02-29 09:30 · Europe/Berlin",
			title: "2028-02-29 09:30 (Europe/Berlin)",
		});
	});

	test("shows a saved one-time schedule without a timezone", () => {
		expect(
			describeEventEntry(event, {
				scheduled_for: { date: "2020-01-01", time: "00:00" },
			}),
		).toEqual({
			text: "2020-01-01 00:00",
			title: "2020-01-01 00:00",
		});
	});

	test("keeps recurring expressions as the schedule when both are present", () => {
		expect(
			describeEventEntry(event, {
				expression: "0 9 * * *",
				scheduled_for: { date: "2028-02-29", time: "09:30" },
			}),
		).toEqual({ text: "0 9 * * *", title: "0 9 * * *" });
	});

	test.each([
		{},
		{ scheduled_for: {} },
		{ scheduled_for: { date: "2027-02-29", time: "09:30" } },
		{ scheduled_for: { date: "2028-02-29", time: "24:00" } },
	])("keeps missing or invalid schedules visible: %j", (config) => {
		expect(describeEventEntry(event, config)).toEqual({
			text: "No schedule",
			muted: true,
		});
	});
});
