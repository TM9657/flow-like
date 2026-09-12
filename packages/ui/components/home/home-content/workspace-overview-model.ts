import type { IExecutionActivity } from "../../../lib/schema/usage/tracking";
import {
	type HomeActivity,
	normalizeHomeActivity,
} from "../home-activity-statistics";

export function workspaceProfileAppCount(
	availableIds: readonly string[] | undefined,
	profileIds: readonly string[] | undefined,
): number | undefined {
	if (availableIds === undefined || profileIds === undefined) return undefined;
	const visible = new Set(profileIds);
	return new Set(availableIds.filter((id) => visible.has(id))).size;
}

export function workspacePulseHistory(
	activity: IExecutionActivity | undefined,
): HomeActivity | null {
	return activity ? normalizeHomeActivity(activity) : null;
}

/** A failed or disabled source must not turn cached history into current metrics. */
export function workspacePulseMetrics(
	history: HomeActivity | null,
	enabled: boolean,
	error: boolean,
): HomeActivity | null {
	return enabled && !error ? history : null;
}

export function workspacePulseState({
	authenticated,
	supported,
	loading,
	error,
	volume,
}: {
	authenticated: boolean;
	supported: boolean;
	loading: boolean;
	error: boolean;
	volume: number | undefined;
}): "starter" | "loading" | "unavailable" | "activity" {
	if (!authenticated || !supported) return "starter";
	if (loading && volume === undefined) return "loading";
	if (error) return "unavailable";
	return volume ? "activity" : "starter";
}
