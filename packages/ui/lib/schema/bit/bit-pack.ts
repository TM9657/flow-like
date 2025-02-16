export interface IBitPack {
    bits: IBit[];
    [property: string]: any;
}

export interface IBit {
    authors:              string[];
    created:              string;
    dependencies:         Array<string[]>;
    dependency_tree_hash: string;
    download_link?:       null | string;
    file_name?:           null | string;
    hash:                 string;
    hub:                  string;
    icon:                 string;
    id:                   string;
    license:              string;
    meta:                 { [key: string]: IBitMeta };
    parameters:           any;
    repository?:          null | string;
    size?:                number | null;
    type:                 IBitTypes;
    updated:              string;
    version:              string;
    [property: string]: any;
}

export interface IBitMeta {
    description:      string;
    long_description: string;
    name:             string;
    tags:             string[];
    use_case:         string;
    [property: string]: any;
}

export enum IBitTypes {
    Board = "Board",
    Config = "Config",
    Course = "Course",
    Embedding = "Embedding",
    File = "File",
    ImageEmbedding = "ImageEmbedding",
    Llm = "Llm",
    Media = "Media",
    Other = "Other",
    PreprocessorConfig = "PreprocessorConfig",
    Project = "Project",
    Projection = "Projection",
    SpecialTokensMap = "SpecialTokensMap",
    Template = "Template",
    Tokenizer = "Tokenizer",
    TokenizerConfig = "TokenizerConfig",
    Vlm = "Vlm",
}
