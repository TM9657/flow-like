import type {
	IEmbeddingUsageRecord,
	IExecutionActivity,
	IExecutionUsageRecord,
	ILlmUsageRecord,
	IPaginatedResponse,
	IUsageSummary,
} from "../../lib/schema/usage/tracking";

export interface IUsageState {
	getLlmHistory(
		page?: number,
		pageSize?: number,
		appId?: string,
	): Promise<IPaginatedResponse<ILlmUsageRecord>>;

	getEmbeddingHistory(
		page?: number,
		pageSize?: number,
		appId?: string,
	): Promise<IPaginatedResponse<IEmbeddingUsageRecord>>;

	getExecutionHistory(
		page?: number,
		pageSize?: number,
		appId?: string,
	): Promise<IPaginatedResponse<IExecutionUsageRecord>>;

	/**
	 * Counts for a whole period rather than a page of it. Prefer this over
	 * paging {@link getExecutionHistory} whenever a caller needs totals, per-day
	 * buckets or a per-app split: a page cannot answer those without dropping
	 * everything past its own size limit.
	 */
	getExecutionActivity(
		days?: number,
		appId?: string,
	): Promise<IExecutionActivity>;

	getUsageSummary(): Promise<IUsageSummary>;
}
