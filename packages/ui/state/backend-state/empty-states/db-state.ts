import type {
	IDatabaseState,
	IIndexConfig,
	IIndexType,
	IQueryTablePayload,
} from "../db-state";

export class EmptyDatabaseState implements IDatabaseState {
	buildIndex(
		appId: string,
		tableName: string,
		column: string,
		indexType: IIndexType,
		optimize?: boolean,
	): Promise<void> {
		throw new Error("Method not implemented.");
	}
	addItems(appId: string, tableName: string, items: any[]): Promise<void> {
		throw new Error("Method not implemented.");
	}
	removeItems(appId: string, tableName: string, query: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
	listItems(
		appId: string,
		tableName: string,
		offset?: number,
		limit?: number,
	): Promise<any[]> {
		throw new Error("Method not implemented.");
	}
	queryItems(
		appId: string,
		tableName: string,
		query: IQueryTablePayload,
		offset?: number,
		limit?: number,
	): Promise<any[]> {
		throw new Error("Method not implemented.");
	}
	getSchema(appId: string, tableName: string): Promise<any> {
		throw new Error("Method not implemented.");
	}
	getIndices(appId: string, tableName: string): Promise<IIndexConfig[]> {
		throw new Error("Method not implemented.");
	}
	listTables(appId: string): Promise<string[]> {
		throw new Error("Method not implemented.");
	}
}
