import { useEffect, useState } from "react";
import { useRunExecutionStore } from "../state/run-execution-state";

export type ActivityStatus = "active" | "stale" | "warning" | "inactive";

interface RunActivity {
	timeSinceLastUpdate: number | null;
	status: ActivityStatus;
	formattedTime: string;
}

/**
 * Get activity status based on time since last update
 * @param timeSinceMs Time in milliseconds since last update
 * @returns Activity status for color coding
 */
export function getActivityStatus(timeSinceMs: number | null): ActivityStatus {
	if (timeSinceMs === null) return "inactive";
	const seconds = timeSinceMs / 1000;
	if (seconds < 30) return "active";
	if (seconds < 60) return "stale";
	return "warning";
}

/**
 * Format time since last update using Intl.RelativeTimeFormat
 * @param timeSinceMs Time in milliseconds since last update
 * @returns Formatted string like "5s ago", "2m ago"
 */
export function formatTimeSinceUpdate(timeSinceMs: number | null): string {
	if (timeSinceMs === null) return "--";

	const seconds = Math.floor(timeSinceMs / 1000);

	// Use Intl.RelativeTimeFormat for localization
	const rtf = new Intl.RelativeTimeFormat("en", { numeric: "always", style: "narrow" });

	if (seconds < 60) {
		return rtf.format(-seconds, "second");
	}

	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) {
		return rtf.format(-minutes, "minute");
	}

	const hours = Math.floor(minutes / 60);
	return rtf.format(-hours, "hour");
}

/**
 * Hook to track run activity with auto-updating time display
 * @param runId Run ID to track
 * @param updateIntervalMs How often to update the display (default 1000ms)
 * @returns Activity information including time, status, and formatted string
 */
export function useRunActivity(
	runId: string | undefined,
	updateIntervalMs: number = 1000,
): RunActivity {
	// Subscribe to the specific run's lastNodeUpdateMs to trigger re-renders
	const lastNodeUpdateMs = useRunExecutionStore((state) => {
		if (!runId) return undefined;
		const run = state.runs.get(runId);
		return run?.lastNodeUpdateMs;
	});

	const [activity, setActivity] = useState<RunActivity>({
		timeSinceLastUpdate: null,
		status: "inactive",
		formattedTime: "--",
	});

	useEffect(() => {
		if (!runId || lastNodeUpdateMs === undefined) {
			setActivity({
				timeSinceLastUpdate: null,
				status: "inactive",
				formattedTime: "--",
			});
			return;
		}

		const updateActivity = () => {
			const timeSince = Date.now() - lastNodeUpdateMs;
			setActivity({
				timeSinceLastUpdate: timeSince,
				status: getActivityStatus(timeSince),
				formattedTime: formatTimeSinceUpdate(timeSince),
			});
		};

		// Update immediately
		updateActivity();

		// Set up interval for continuous updates
		const interval = setInterval(updateActivity, updateIntervalMs);

		return () => clearInterval(interval);
	}, [runId, lastNodeUpdateMs, updateIntervalMs]);

	return activity;
}

/**
 * Get CSS class names for activity status color coding
 * @param status Activity status
 * @returns Tailwind class names for text and background colors
 */
export function getActivityColorClasses(status: ActivityStatus): {
	text: string;
	bg: string;
	border: string;
} {
	switch (status) {
		case "active":
			return {
				text: "text-green-600 dark:text-green-400",
				bg: "bg-green-500/20",
				border: "border-green-500",
			};
		case "stale":
			return {
				text: "text-yellow-600 dark:text-yellow-400",
				bg: "bg-yellow-500/20",
				border: "border-yellow-500",
			};
		case "warning":
			return {
				text: "text-red-600 dark:text-red-400",
				bg: "bg-red-500/20",
				border: "border-red-500",
			};
		case "inactive":
		default:
			return {
				text: "text-muted-foreground",
				bg: "bg-muted",
				border: "border-muted",
			};
	}
}
