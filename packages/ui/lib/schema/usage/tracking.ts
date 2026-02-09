export interface ILlmUsageRecord {
	id: string;
	model_id: string;
	token_in: number;
	token_out: number;
	latency: number | null;
	app_id: string | null;
	price: number;
	created_at: string;
}

export interface IEmbeddingUsageRecord {
	id: string;
	model_id: string;
	token_count: number;
	latency: number | null;
	app_id: string | null;
	price: number;
	created_at: string;
}

export interface IExecutionUsageRecord {
	id: string;
	instance: string | null;
	board_id: string;
	node_id: string;
	version: string;
	microseconds: number;
	status: string;
	app_id: string | null;
	created_at: string;
}

export interface IPaginatedResponse<T> {
	items: T[];
	total: number;
	page: number;
	page_size: number;
}

export interface IUsageSummary {
	total_llm_price: number;
	total_embedding_price: number;
	total_llm_invocations: number;
	total_embedding_invocations: number;
	total_executions: number;
}
