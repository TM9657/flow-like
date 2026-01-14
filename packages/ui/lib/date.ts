import type { IDate } from "../types";

export function formatRelativeTime(
	dateInput: IDate | string,
	style: Intl.RelativeTimeFormatStyle = "long"
) {
	// 1. Normalize the input to milliseconds
	let targetTimeMs: number;

	if (typeof dateInput === 'string') {
		targetTimeMs = new Date(dateInput).getTime();
	} else {
		// Handle IDate
		targetTimeMs = dateInput.secs_since_epoch * 1000;
	}

	// Safety check: if string was invalid, getTime() returns NaN
	if (isNaN(targetTimeMs)) {
		return "Invalid date";
	}

	// 2. Calculate units
	const diffMilliseconds = Date.now() - targetTimeMs;
	const seconds = Math.round(diffMilliseconds / 1000);
	const minutes = Math.round(seconds / 60);
	const hours = Math.round(minutes / 60);
	const days = Math.round(hours / 24);

	const formatter = new Intl.RelativeTimeFormat(undefined, {
		numeric: "auto",
		style: style,
	});

	// 3. Format with Math.abs() to handle future dates safely
	if (Math.abs(seconds) < 60) {
		return formatter.format(-seconds, "second");
	}
	if (Math.abs(minutes) < 60) {
		return formatter.format(-minutes, "minute");
	}
	if (Math.abs(hours) < 24) {
		return formatter.format(-hours, "hour");
	}
	return formatter.format(-days, "day");
}

export function parseTimespan(start: IDate, end: IDate) {
	if (start.nanos_since_epoch > end.nanos_since_epoch) {
		const old_end = end;
		end = start;
		start = old_end;
	}

	const diff = end.nanos_since_epoch - start.nanos_since_epoch;
	const μs = diff / 1000;

	if (μs < 1000) return `${μs.toFixed(2)}μs`;
	const ms = μs / 1000;
	if (ms < 1000) return `${ms.toFixed(2)}ms`;
	const s = ms / 1000;
	if (s < 60) return `${s.toFixed(2)}s`;
	const m = s / 60;
	if (m < 60) return `${m.toFixed(2)}m`;
	const h = m / 60;
	if (h < 24) return `${h.toFixed(2)}h`;
	const d = h / 24;
	return `${d.toFixed(2)}d`;
}

export function formatDuration(μs: number) {
	if (μs < 1000) return `${μs.toFixed(2)} μs`;
	const ms = μs / 1000;
	if (ms < 1000) return `${ms.toFixed(2)} ms`;
	const s = ms / 1000;
	if (s < 60) return `${s.toFixed(2)} s`;
	const m = s / 60;
	if (m < 60) return `${m.toFixed(2)} m`;
	const h = m / 60;
	if (h < 24) return `${h.toFixed(2)} h`;
	const d = h / 24;
	return `${d.toFixed(2)}d`;
}
