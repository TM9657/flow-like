export interface IImageEmbeddingModelParameters {
    languages:     string[];
    pooling:       IPooling;
    provider:      IBitProviderModel;
    vector_length: number;
    [property: string]: any;
}

export enum IPooling {
    Cls = "CLS",
    Mean = "Mean",
    None = "None",
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
