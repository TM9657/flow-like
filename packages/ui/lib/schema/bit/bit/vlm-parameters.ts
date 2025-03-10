export interface IVlmParameters {
    context_length:       number;
    model_classification: IBitModelClassification;
    provider:             IBitProviderModel;
    [property: string]: any;
}

export interface IBitModelClassification {
    coding:           number;
    cost:             number;
    creativity:       number;
    factuality:       number;
    function_calling: number;
    multilinguality:  number;
    openness:         number;
    reasoning:        number;
    safety:           number;
    speed:            number;
    [property: string]: any;
}

export interface IBitProviderModel {
    model_id?:     null | string;
    provider_name: IBitProvider;
    version?:      null | string;
    [property: string]: any;
}

export enum IBitProvider {
    Anthropic = "Anthropic",
    AzureOpenAI = "AzureOpenAI",
    Bedrock = "Bedrock",
    Local = "Local",
    OpenAI = "OpenAI",
}
