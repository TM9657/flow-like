export interface IVlmParameters {
	context_length: number;
	model_classification: IModelClassification;
	provider: IProvider;
	[property: string]: any;
}

export interface IModelClassification {
	coding: number;
	cost: number;
	creativity: number;
	factuality: number;
	function_calling: number;
	multilinguality: number;
	openness: number;
	reasoning: number;
	safety: number;
	speed: number;
	[property: string]: any;
}

export interface IProvider {
	model_id?: null | string;
	params?: { [key: string]: any } | null;
	provider_name: string;
	version?: null | string;
	[property: string]: any;
}
