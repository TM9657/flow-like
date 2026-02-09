import type {
	IEmbeddingUsageRecord,
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

	getUsageSummary(): Promise<IUsageSummary>;
}
