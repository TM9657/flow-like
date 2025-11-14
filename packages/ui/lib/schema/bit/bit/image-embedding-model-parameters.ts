export interface IImageEmbeddingModelParameters {
	languages: string[];
	pooling: IPooling;
	provider: IProvider;
	vector_length: number;
	[property: string]: any;
}

export enum IPooling {
	Cls = "CLS",
	Mean = "Mean",
	None = "None",
}

export interface IProvider {
	model_id?: null | string;
	params?: { [key: string]: any } | null;
	provider_name: string;
	version?: null | string;
	[property: string]: any;
}
