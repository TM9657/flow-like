import type { ForecastTimestampFormat } from "../../../../state/backend-state/query-state";

export const timestampFormatOptions: {
	value: ForecastTimestampFormat;
	label: string;
}[] = [
	{ value: "auto", label: "Detect automatically" },
	{ value: "unix_seconds", label: "Unix seconds" },
	{ value: "unix_milliseconds", label: "Unix milliseconds" },
	{ value: "unix_microseconds", label: "Unix microseconds" },
	{ value: "unix_nanoseconds", label: "Unix nanoseconds" },
	{ value: "day_first", label: "Day first (31/12/2026)" },
	{ value: "month_first", label: "Month first (12/31/2026)" },
];

export const numericTextPattern =
	/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/;
const shifts: Partial<Record<ForecastTimestampFormat, number>> = {
	unix_seconds: 3,
	unix_milliseconds: 0,
	unix_microseconds: -3,
	unix_nanoseconds: -6,
};
const months = [
	"jan",
	"feb",
	"mar",
	"apr",
	"may",
	"jun",
	"jul",
	"aug",
	"sep",
	"oct",
	"nov",
	"dec",
];

const fullMonths = [
	"january",
	"february",
	"march",
	"april",
	"may",
	"june",
	"july",
	"august",
	"september",
	"october",
	"november",
	"december",
];
function monthNumber(value: string): number {
	const normalized = value.toLowerCase();
	const index =
		normalized.length === 3
			? months.indexOf(normalized)
			: fullMonths.indexOf(normalized);
	return index + 1;
}

export function nativeTimestampUnit(typeName: string): string | undefined {
	if (/^date32\b/i.test(typeName)) return "days";
	if (/^date64\b/i.test(typeName)) return "milliseconds";
	return /^timestamp\(\s*(second|millisecond|microsecond|nanosecond|s|ms|us|ns)\b/i
		.exec(typeName)?.[1]
		?.toLowerCase();
}

function inferredEpoch(text: string): ForecastTimestampFormat {
	const magnitude = Math.abs(Number(text));
	if (magnitude >= 1e8 && magnitude < 1e11) return "unix_seconds";
	if (magnitude >= 1e11 && magnitude < 1e14) return "unix_milliseconds";
	if (magnitude >= 1e14 && magnitude < 1e17) return "unix_microseconds";
	if (magnitude >= 1e17 && magnitude < 1e20) return "unix_nanoseconds";
	throw new Error("Unix timestamp unit is ambiguous. Choose its unit below.");
}

function epochMilliseconds(text: string, shift: number): number {
	if (text.length > 128 || !numericTextPattern.test(text))
		throw new Error("Invalid numeric timestamp.");
	const [mantissa, exponent = "0"] = text.toLowerCase().split("e");
	const decimals = mantissa.split(".")[1]?.length ?? 0;
	const power = Number(exponent) - decimals + shift;
	if (!Number.isSafeInteger(power) || Math.abs(power) > 40)
		throw new Error("Timestamp exceeds the supported precision.");
	const coefficient = BigInt(mantissa.replace(".", ""));
	const scale = 10n ** BigInt(Math.abs(power));
	if (power < 0 && coefficient % scale !== 0n)
		throw new Error("Timestamps must have whole-millisecond precision.");
	const time = Number(power < 0 ? coefficient / scale : coefficient * scale);
	if (!Number.isSafeInteger(time))
		throw new Error("Timestamp exceeds the supported range.");
	return time;
}

function calendarTime(
	year: number,
	month: number,
	day: number,
	tail = "",
): number {
	const match =
		/^(?:[ T](\d{1,2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?)?\s*(Z|[+-]\d{2}:?\d{2})?$/i.exec(
			tail,
		);
	if (!match) throw new Error("Unsupported time format. Use ISO 8601.");
	const [, hour = "0", minute = "0", second = "0", fraction = "", zone] = match;
	if (/[1-9]/.test(fraction.slice(3)))
		throw new Error("Timestamps must have whole-millisecond precision.");
	const date = new Date(0);
	date.setUTCFullYear(year, month - 1, day);
	date.setUTCHours(
		Number(hour),
		Number(minute),
		Number(second),
		Number(fraction.slice(0, 3).padEnd(3, "0")),
	);
	if (
		date.getUTCFullYear() !== year ||
		date.getUTCMonth() !== month - 1 ||
		date.getUTCDate() !== day ||
		Number(hour) > 23 ||
		Number(minute) > 59 ||
		Number(second) > 59
	)
		throw new Error("Invalid calendar date or time.");
	let offset = 0;
	if (zone && zone.toUpperCase() !== "Z") {
		const digits = zone.slice(1).replace(":", "");
		const hours = Number(digits.slice(0, 2));
		const minutes = Number(digits.slice(2));
		if (hours > 23 || minutes > 59) throw new Error("Invalid UTC offset.");
		offset = (hours * 60 + minutes) * 60000 * (zone[0] === "-" ? -1 : 1);
	}
	return date.getTime() - offset;
}

function dateTimestamp(text: string, format?: ForecastTimestampFormat): number {
	let match = /^(\d{4})[-/](\d{1,2})[-/](\d{1,2})(.*)$/.exec(text);
	if (match)
		return calendarTime(
			Number(match[1]),
			Number(match[2]),
			Number(match[3]),
			match[4],
		);
	match = /^(\d{1,2})([/.-])(\d{1,2})\2(\d{4})(.*)$/.exec(text);
	if (match) {
		const first = Number(match[1]);
		const second = Number(match[3]);
		if (
			first <= 12 &&
			second <= 12 &&
			first !== second &&
			format !== "day_first" &&
			format !== "month_first"
		)
			throw new Error(
				"Date order is ambiguous. Choose day first or month first.",
			);
		const dayFirst =
			format === "day_first" || (format !== "month_first" && first > 12);
		return calendarTime(
			Number(match[4]),
			dayFirst ? second : first,
			dayFirst ? first : second,
			match[5],
		);
	}
	match = /^(\d{1,2})[ -]([a-z]+)[ -](\d{4})(.*)$/i.exec(text);
	if (match)
		return calendarTime(
			Number(match[3]),
			monthNumber(match[2]),
			Number(match[1]),
			match[4],
		);
	match = /^([a-z]+) (\d{1,2}), (\d{4})(.*)$/i.exec(text);
	if (match)
		return calendarTime(
			Number(match[3]),
			monthNumber(match[1]),
			Number(match[2]),
			match[4],
		);
	// Restrict the browser parser to the explicit RFC 2822 grammar with a timezone.
	match =
		/^(?:[a-z]{3},\s*)?(\d{1,2}) ([a-z]{3}) (\d{4}) (\d{2}:\d{2}(?::\d{2})?) (GMT|UT|[+-]\d{4})$/i.exec(
			text,
		);
	if (match)
		return calendarTime(
			Number(match[3]),
			monthNumber(match[2]),
			Number(match[1]),
			` ${match[4]}${/^(GMT|UT)$/i.test(match[5]) ? "Z" : match[5]}`,
		);
	throw new Error(
		"Unsupported date format. Use ISO 8601 or choose its date order.",
	);
}

function parse(
	value: unknown,
	typeName: string,
	format?: ForecastTimestampFormat,
): {
	timestamp: number;
	format?: ForecastTimestampFormat;
	representation: string;
} {
	if (value === null || value === undefined)
		throw new Error(
			"Timestamp is missing. Filter or fill missing timestamps first.",
		);
	if (typeof value !== "number" && typeof value !== "string")
		throw new Error("Timestamp must contain dates or numeric Unix epochs.");
	if (
		typeof value === "number" &&
		(!Number.isFinite(value) || Math.abs(value) > Number.MAX_SAFE_INTEGER)
	)
		throw new Error(
			"Timestamp has lost numeric precision. Use an integer or text column.",
		);
	const text = String(value).trim();
	const numeric = numericTextPattern.test(text);
	const unit = nativeTimestampUnit(typeName);
	if (/^timestamp\b/i.test(typeName) && !unit)
		throw new Error("Unknown native timestamp unit.");
	if (unit && numeric) {
		if (
			!/^[+-]?\d+$/.test(text) ||
			BigInt(text) < -9223372036854775808n ||
			BigInt(text) > 9223372036854775807n
		)
			throw new Error("Native timestamp exceeds its integer storage range.");
		const nativeShift = (
			{
				s: 3,
				second: 3,
				ms: 0,
				millisecond: 0,
				milliseconds: 0,
				us: -3,
				microsecond: -3,
				ns: -6,
				nanosecond: -6,
			} as Record<string, number>
		)[unit];
		const timestamp =
			unit === "days"
				? epochMilliseconds(text, 0) * 86400000
				: epochMilliseconds(text, nativeShift);
		if (!Number.isSafeInteger(timestamp))
			throw new Error("Timestamp exceeds the supported range.");
		return { timestamp, representation: "native" };
	}
	if (numeric) {
		if (format === "day_first" || format === "month_first")
			throw new Error("The selected date order requires date text.");
		const resolved =
			format && format !== "auto"
				? format
				: !format
					? "unix_milliseconds"
					: inferredEpoch(text);
		return {
			timestamp: epochMilliseconds(text, shifts[resolved] ?? 0),
			format: resolved,
			representation: typeof value === "number" ? "number" : "numeric text",
		};
	}
	if (format?.startsWith("unix_") && !unit)
		throw new Error("The selected Unix unit requires numeric timestamps.");
	return {
		timestamp: dateTimestamp(text, format),
		representation: "date text",
	};
}

export function parseForecastTimestamp(
	value: unknown,
	typeName = "Int64",
	format?: ForecastTimestampFormat,
): number | null {
	try {
		return parse(value, typeName, format).timestamp;
	} catch {
		return null;
	}
}

export function inspectForecastTimestamps(
	values: readonly unknown[],
	typeName: string,
	format?: ForecastTimestampFormat,
): {
	format?: ForecastTimestampFormat;
	label: string;
	samples: { raw: string; timestamp: number }[];
	error?: string;
} {
	const samples: { raw: string; timestamp: number }[] = [];
	let resolved: ForecastTimestampFormat | undefined;
	let representation: string | undefined;
	for (const [index, value] of values.entries()) {
		try {
			const parsed = parse(value, typeName, format);
			if (representation && representation !== parsed.representation)
				throw new Error(
					"Timestamp column mixes numeric and text representations.",
				);
			if (resolved && parsed.format && resolved !== parsed.format)
				throw new Error(
					"Timestamp column mixes Unix units. Normalize the column first.",
				);
			resolved = parsed.format;
			representation = parsed.representation;
			if (samples.length < 3)
				samples.push({ raw: String(value), timestamp: parsed.timestamp });
		} catch (error) {
			return {
				label: "Check timestamp format",
				samples,
				error: `Preview row ${index + 1}: ${error instanceof Error ? error.message : String(error)}`,
			};
		}
	}
	const label = nativeTimestampUnit(typeName)
		? "Native timestamp/date"
		: (timestampFormatOptions.find((option) => option.value === resolved)
				?.label ??
			(format === "day_first"
				? "Day-first dates"
				: format === "month_first"
					? "Month-first dates"
					: "Date text"));
	return { format: resolved, label, samples };
}
