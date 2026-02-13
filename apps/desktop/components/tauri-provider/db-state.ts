import { invoke } from "@tauri-apps/api/core";
import type {
	IAddColumnPayload,
	IDatabaseState,
	IIndexConfig,
	IIndexType,
	IQueryTablePayload,
} from "@tm9657/flow-like-ui";
import { fetcher } from "../../lib/api";
import type { TauriBackend } from "../tauri-provider";

function parseTableName(name: string): string {
	return encodeURIComponent(name);
}

export class DatabaseState implements IDatabaseState {
	constructor(private readonly backend: TauriBackend) {}
	async buildIndex(
		appId: string,
		tableName: string,
		column: string,
		indexType: IIndexType,
		optimize?: boolean,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/index`,
				{
					method: "POST",
					body: JSON.stringify({
						column,
						indexType,
						optimize,
					}),
				},
				this.backend.auth,
			);
		}

		return await invoke("build_index", {
			appId,
			tableName,
			column,
			indexType,
			_optimize: optimize,
		});
	}

	async addItems(
		appId: string,
		tableName: string,
		items: any[],
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}`,
				{
					method: "PUT",
					body: JSON.stringify({
						items,
					}),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_add", { appId, tableName, items });
	}

	async removeItems(
		appId: string,
		tableName: string,
		query: string,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}`,
				{
					method: "DELETE",
					body: JSON.stringify({
						query,
					}),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_delete", { appId, tableName, query });
	}

	async listItems(
		appId: string,
		tableName: string,
		offset?: number,
		limit?: number,
	): Promise<any[]> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}?offset=${offset ?? 0}&limit=${limit ?? 25}`,
				{
					method: "GET",
				},
				this.backend.auth,
			);
		}

		return await invoke("db_list", { appId, tableName, offset, limit });
	}

	async queryItems(
		appId: string,
		tableName: string,
		query: IQueryTablePayload,
		offset?: number,
		limit?: number,
	): Promise<any[]> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/query?offset=${offset ?? 0}&limit=${limit ?? 25}`,
				{
					method: "POST",
					body: JSON.stringify(query),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_query", {
			appId,
			tableName,
			payload: query,
			offset,
			limit,
		});
	}

	async getSchema(appId: string, tableName: string): Promise<any> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/schema`,
				{
					method: "GET",
				},
				this.backend.auth,
			);
		}

		return await invoke<any>("db_schema", {
			appId,
			tableName,
		});
	}

	async getIndices(appId: string, tableName: string): Promise<IIndexConfig[]> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/indices`,
				{
					method: "GET",
				},
				this.backend.auth,
			);
		}

		return await invoke("db_indices", { appId, tableName });
	}

	async dropIndex(
		appId: string,
		tableName: string,
		indexName: string,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/indices/${encodeURIComponent(indexName)}`,
				{
					method: "DELETE",
				},
				this.backend.auth,
			);
			return;
		}

		await invoke("db_drop_index", { appId, tableName, indexName });
	}

	async listTables(appId: string): Promise<string[]> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db`,
				{
					method: "GET",
				},
				this.backend.auth,
			);
		}

		return await invoke("db_table_names", { appId });
	}

	async countItems(appId: string, tableName: string): Promise<number> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/count`,
				{
					method: "GET",
				},
				this.backend.auth,
			);
		}

		return await invoke("db_count", { appId, tableName });
	}

	async optimize(
		appId: string,
		tableName: string,
		keepVersions?: boolean,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/optimize`,
				{
					method: "POST",
					body: JSON.stringify({ keepVersions: keepVersions ?? false }),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_optimize", {
			appId,
			tableName,
			keepVersions: keepVersions ?? false,
		});
	}

	async updateItem(
		appId: string,
		tableName: string,
		filter: string,
		updates: Record<string, any>,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/update`,
				{
					method: "POST",
					body: JSON.stringify({ filter, updates }),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_update", { appId, tableName, filter, updates });
	}

	async dropColumns(
		appId: string,
		tableName: string,
		columns: string[],
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/columns`,
				{
					method: "DELETE",
					body: JSON.stringify({ columns }),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_drop_columns", { appId, tableName, columns });
	}

	async addColumn(
		appId: string,
		tableName: string,
		column: IAddColumnPayload,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/columns`,
				{
					method: "POST",
					body: JSON.stringify(column),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_add_column", { appId, tableName, column });
	}

	async alterColumn(
		appId: string,
		tableName: string,
		column: string,
		nullable: boolean,
	): Promise<void> {
		const isOffline = await this.backend.isOffline(appId);

		if (!isOffline) {
			return await fetcher(
				this.backend.profile!,
				`apps/${appId}/db/${parseTableName(tableName)}/columns/alter`,
				{
					method: "POST",
					body: JSON.stringify({ column, nullable }),
				},
				this.backend.auth,
			);
		}

		return await invoke("db_alter_column", {
			appId,
			tableName,
			column,
			nullable,
		});
	}
}
