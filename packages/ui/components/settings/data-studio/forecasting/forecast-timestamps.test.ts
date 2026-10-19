import { describe, expect, it } from "bun:test";
import {
	inspectForecastTimestamps,
	parseForecastTimestamp as parse,
} from "./forecast-timestamps";

describe("forecast timestamp interpretation", () => {
	const epoch = Date.UTC(2026, 9, 19, 12, 30);
	it("detects all four Unix units, including exact numeric text", () => {
		for (const [value, format] of [
			[epoch / 1000, "unix_seconds"],
			[epoch, "unix_milliseconds"],
			[String(BigInt(epoch) * 1000n), "unix_microseconds"],
			[String(BigInt(epoch) * 1000000n), "unix_nanoseconds"],
		] as const) {
			expect(parse(value, "LargeUtf8", "auto")).toBe(epoch);
			expect(
				inspectForecastTimestamps([value], "LargeUtf8", "auto").format,
			).toBe(format);
		}
		expect(parse("1792413000.125", "Utf8", "auto")).toBe(1792413000125);
		expect(parse("1.792413e9", "Utf8", "auto")).toBe(1792413000000);
	});
	it("requires units for small epochs while retaining saved legacy semantics", () => {
		expect(parse(10, "Int64", "auto")).toBeNull();
		expect(parse(10, "Int64", "unix_seconds")).toBe(10000);
		expect(parse(10, "Int64")).toBe(10);
	});
	it("honors Arrow units including DataFusion short names", () => {
		expect(parse(10, 'Timestamp(ms, "UTC")', "auto")).toBe(10);
		expect(parse(10, "Timestamp(Second, None)", "auto")).toBe(10000);
		expect(parse("2000000", 'Timestamp(ns, "UTC")', "unix_seconds")).toBe(2);
		expect(parse(1, "Date32", "auto")).toBe(86400000);
		expect(parse(10, "Date64", "auto")).toBe(10);
	});
	it("normalizes common date formats and UTC offsets deterministically", () => {
		for (const text of [
			"2026-10-19T12:30:00Z",
			"2026/10/19 12:30",
			"2026-10-19 14:30:00+02:00",
			"19 Oct 2026 12:30",
			"October 19, 2026 12:30",
			"Mon, 19 Oct 2026 12:30:00 +0000",
		])
			expect(parse(text, "Utf8", "auto")).toBe(epoch);
		expect(parse("2026-10-19", "Utf8", "auto")).toBe(Date.UTC(2026, 9, 19));
	});
	it("requires an explicit date order when both readings are valid", () => {
		expect(parse("04/05/2026", "Utf8", "auto")).toBeNull();
		expect(parse("04/05/2026", "Utf8", "day_first")).toBe(Date.UTC(2026, 4, 4));
		expect(parse("04/05/2026", "Utf8", "month_first")).toBe(
			Date.UTC(2026, 3, 5),
		);
		expect(parse("19.10.2026", "Utf8", "auto")).toBe(Date.UTC(2026, 9, 19));
		expect(parse("10/19/2026", "Utf8", "auto")).toBe(Date.UTC(2026, 9, 19));
	});
	it("rejects invalid dates, missing values and lost precision", () => {
		for (const value of [
			"2026-02-30",
			"2026-10-19T25:00:00Z",
			"2026-10-19T12:30:00.0001Z",
			"1792413000.0001",
			"Infinity",
			"",
			null,
			Number.MAX_SAFE_INTEGER + 1,
		])
			expect(parse(value, "Utf8", "auto")).toBeNull();
	});
	it("reports mixed units and representations with the preview row", () => {
		expect(
			inspectForecastTimestamps([epoch, epoch / 1000], "Int64", "auto").error,
		).toContain("row 2");
		expect(
			inspectForecastTimestamps(["2026-10-19", String(epoch)], "Utf8", "auto")
				.error,
		).toContain("mixes");
		expect(
			inspectForecastTimestamps([epoch, null], "Int64", "auto").error,
		).toContain("missing");
	});
});
