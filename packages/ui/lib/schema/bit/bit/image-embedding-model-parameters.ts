export interface IImageEmbeddingModelParameters {
	languages: string[];
	pooling: IPooling;
	provider: IModelProvider;
	vector_length: number;
	[property: string]: any;
}

export enum IPooling {
	Cls = "CLS",
	Mean = "Mean",
	None = "None",
}

export interface IModelProvider {
	model_id?: null | string;
	provider_name: string;
	version?: null | string;
	[property: string]: any;
}
