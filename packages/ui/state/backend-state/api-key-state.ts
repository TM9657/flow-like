export interface ITechnicalUser {
	id: string;
	name: string;
	description?: string;
	role_id?: string;
	role_name?: string;
	role_permissions?: bigint;
	valid_until?: number;
	created_at: number;
}

export interface ITechnicalUserCreateInput {
	name: string;
	description?: string;
	role_id?: string;
	valid_until?: number;
}

export interface ITechnicalUserCreateResult {
	id: string;
	api_key: string;
	name: string;
	role_name?: string;
}

export interface IApiKeyState {
	getApiKeys(appId: string): Promise<ITechnicalUser[]>;
	createApiKey(
		appId: string,
		input: ITechnicalUserCreateInput,
	): Promise<ITechnicalUserCreateResult>;
	deleteApiKey(appId: string, keyId: string): Promise<void>;
}
