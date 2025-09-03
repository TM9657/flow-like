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
	countItems(
		appId: string,
		tableName: string,
	): Promise<number>;
	getSchema(appId: string, tableName: string): Promise<any>;
	getIndices(appId: string, tableName: string): Promise<IIndexConfig[]>;
	listTables(appId: string): Promise<string[]>;
}
