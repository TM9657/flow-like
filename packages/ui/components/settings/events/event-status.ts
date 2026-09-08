export type EventStatus = "live" | "paused" | "attention" | "unknown";

export function getEventStatus({
	active,
	blocking,
	requiresSink,
	sinkActive,
}: {
	active: boolean;
	blocking: boolean;
	requiresSink: boolean;
	sinkActive?: boolean;
}): EventStatus {
	if (blocking) return "attention";
	if (!active) return "paused";
	if (!requiresSink) return "live";
	if (sinkActive === undefined) return "unknown";
	return sinkActive ? "live" : "attention";
}
