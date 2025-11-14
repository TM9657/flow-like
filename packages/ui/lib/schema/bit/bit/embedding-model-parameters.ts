export interface IEmbeddingModelParameters {
	input_length: number;
	languages: string[];
	pooling: IPooling;
	prefix: IPrefix;
	provider: IProvider;
	vector_length: number;
	[property: string]: any;
}

export enum IPooling {
	Cls = "CLS",
	Mean = "Mean",
	None = "None",
}

export interface IPrefix {
	paragraph: string;
	query: string;
	[property: string]: any;
}

export interface IProvider {
	model_id?: null | string;
	params?: { [key: string]: any } | null;
	provider_name: string;
	version?: null | string;
	[property: string]: any;
}
