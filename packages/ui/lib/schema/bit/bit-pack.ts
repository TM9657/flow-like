export interface IBitPack {
	bits: IBitElement[];
	[property: string]: any;
}

export interface IBitElement {
	authors: string[];
	created: string;
	dependencies: string[];
	dependency_tree_hash: string;
	download_link?: null | string;
	file_name?: null | string;
	hash: string;
	hub: string;
	id: string;
	license?: null | string;
	meta: { [key: string]: IMetaValue };
	parameters: any;
	repository?: null | string;
	size?: number | null;
	type: IType;
	updated: string;
	version?: null | string;
	[property: string]: any;
}

export interface IMetaValue {
	age_rating?: number | null;
	created_at: ICreatedAt;
	description: string;
	docs_url?: null | string;
	icon?: null | string;
	long_description?: null | string;
	name: string;
	organization_specific_values?: number[] | null;
	preview_media: string[];
	release_notes?: null | string;
	support_url?: null | string;
	tags: string[];
	thumbnail?: null | string;
	updated_at: ICreatedAt;
	use_case?: null | string;
	website?: null | string;
	[property: string]: any;
}

export interface ICreatedAt {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}

export enum IType {
	Board = "Board",
	Config = "Config",
	Course = "Course",
	Embedding = "Embedding",
	File = "File",
	ImageEmbedding = "ImageEmbedding",
	Llm = "Llm",
	Media = "Media",
	ObjectDetection = "ObjectDetection",
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
