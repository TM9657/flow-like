import type {
	IExecutionActivity,
	IExecutionUsageRecord,
} from "../../lib/schema/usage/tracking";

export function homeActivityDays(value: unknown): 1 | 7 | 30 {
	return value === 1 || value === 30 ? value : 7;
}

export function hasAttentionSeverity(status: string): boolean {
	return ["error", "fatal"].includes(status.toLowerCase());
}

export function homeActivityPeriod(days: number): string {
	return days === 1 ? "Today" : `Last ${days} days`;
}

export interface HomeActivityBucket {
	day: string;
	count: number;
	attentionCount: number;
}

export interface HomeAppActivity {
	appId: string | null;
	count: number;
	attentionCount: number;
}

/**
 * The home widgets' view of {@link IExecutionActivity}.
 *
 * Every number here is a count of the whole period, produced by the server. It
 * replaces an earlier model that bucketed a single page of the newest records:
 * that could only ever report the page size, so a busy account saw its entire
 * week collapse onto today and every earlier day read as zero.
 */
export interface HomeActivity {
	days: number;
	buckets: HomeActivityBucket[];
	apps: HomeAppActivity[];
	/** Records in the period. */
	total: number;
	/** Records in the period with Error or Fatal severity. */
	attentionTotal: number;
	averageMicroseconds: number | null;
	/** Newest flagged records, capped by the server for display. */
	attention: IExecutionUsageRecord[];
	/** True when more flagged records exist than {@link attention} carries. */
	attentionCapped: boolean;
}

export function normalizeHomeActivity(
	response: IExecutionActivity,
): HomeActivity {
	const attention = response.attention ?? [];
	return {
		days: response.days,
		buckets: (response.buckets ?? []).map((bucket) => ({
			day: bucket.day,
			count: bucket.count,
			attentionCount: bucket.attention_count,
		})),
		apps: (response.apps ?? []).map((app) => ({
			appId: app.app_id,
			count: app.count,
			attentionCount: app.attention_count,
		})),
		total: response.total,
		attentionTotal: response.attention_total,
		averageMicroseconds: response.average_microseconds,
		attention,
		attentionCapped: response.attention_total > attention.length,
	};
}

export function homeActivityCoverage(activity: HomeActivity): string {
	const period =
		activity.days === 1 ? "Today (UTC)" : `Last ${activity.days} days (UTC)`;
	return `${period}: ${activity.total.toLocaleString()} execution ${
		activity.total === 1 ? "record" : "records"
	}, counted in full rather than sampled. ${activity.attentionTotal.toLocaleString()} carried Error or Fatal severity.`;
}

/** Source line for a widget footer: what was counted, over what period. */
export function homeActivitySourceLabel(activity: HomeActivity): string {
	return `Your account · ${activity.total.toLocaleString()} ${
		activity.total === 1 ? "record" : "records"
	} · ${homeActivityPeriod(activity.days).toLowerCase()} (UTC)`;
}

export function homeUsageDollars(microDollars: number): string {
	return new Intl.NumberFormat(undefined, {
		style: "currency",
		currency: "USD",
		minimumFractionDigits: 2,
		maximumFractionDigits: 4,
	}).format(microDollars / 1_000_000);
}

export function homeDurationMs(microseconds: number | null): string {
	if (microseconds === null) return "No records";
	return `${(microseconds / 1000).toLocaleString(undefined, {
		maximumFractionDigits: 0,
	})} ms`;
}
