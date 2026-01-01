export enum IIndexType {
	FullText = 0,
	BTree = 1,
	Bitmap = 2,
	LabelList = 3,
	Auto = 4,
}

export interface IQueryTableVectorPayload {
	column: string;
	vector: number[];
}

export interface IQueryTablePayload {
	sql?: string;
	vector_query?: IQueryTableVectorPayload;
	filter?: string;
	fts_term?: string;
	rerank?: boolean;
}

export interface IIndexConfig {
	name: string;
	index_type: string;
	columns: string[];
}

export interface IAddColumnPayload {
	name: string;
	sql_expression: string;
}

export interface IDatabaseState {
	buildIndex(
		appId: string,
		tableName: string,
		column: string,
		indexType: IIndexType,
		optimize?: boolean,
	): Promise<void>;
	addItems(appId: string, tableName: string, items: any[]): Promise<void>;
	removeItems(appId: string, tableName: string, query: string): Promise<void>;
	listItems(
		appId: string,
		tableName: string,
		offset?: number,
		limit?: number,
	): Promise<any[]>;
	queryItems(
		appId: string,
		tableName: string,
		query: IQueryTablePayload,
		offset?: number,
		limit?: number,
	): Promise<any[]>;
	countItems(appId: string, tableName: string): Promise<number>;
	getSchema(appId: string, tableName: string): Promise<any>;
	getIndices(appId: string, tableName: string): Promise<IIndexConfig[]>;
	dropIndex(appId: string, tableName: string, indexName: string): Promise<void>;
	listTables(appId: string): Promise<string[]>;
	optimize(
		appId: string,
		tableName: string,
		keepVersions?: boolean,
	): Promise<void>;
	updateItem(
		appId: string,
		tableName: string,
		filter: string,
		updates: Record<string, any>,
	): Promise<void>;
	dropColumns(
		appId: string,
		tableName: string,
		columns: string[],
	): Promise<void>;
	addColumn(
		appId: string,
		tableName: string,
		column: IAddColumnPayload,
	): Promise<void>;
	alterColumn(
		appId: string,
		tableName: string,
		column: string,
		nullable: boolean,
	): Promise<void>;
}
