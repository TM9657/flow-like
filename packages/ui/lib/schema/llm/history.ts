export interface IHistory {
	frequency_penalty?: number | null;
	max_completion_tokens?: number | null;
	messages: IHistoryMessage[];
	model: string;
	n?: number | null;
	presence_penalty?: number | null;
	response_format?: any;
	seed?: number | null;
	stop?: string[] | null;
	stream?: boolean | null;
	stream_options?: null | IStreamOptions;
	temperature?: number | null;
	tool_choice?: null | IToolChoice;
	tools?: ITool[] | null;
	top_p?: number | null;
	user?: null | string;
	[property: string]: any;
}

export interface IHistoryMessage {
	content: IContent[];
	role: IRole;
	[property: string]: any;
}

export interface IContent {
	text?: string;
	type: IContentType;
	data?: string;
	mime_type?: string;
	[property: string]: any;
}

export enum IContentType {
	ImageURL = "image_url",
	Text = "text",
}

export enum IRole {
	Assistant = "assistant",
	System = "system",
	User = "user",
}

export interface IStreamOptions {
	include_usage: boolean;
	[property: string]: any;
}

export interface IToolChoice {
	function: IHistoryFunction;
	type: IToolType;
	[property: string]: any;
}

export interface IHistoryFunction {
	description?: null | string;
	name: string;
	parameters?: any;
	strict?: boolean | null;
	[property: string]: any;
}

export enum IToolType {
	Function = "function",
}

export interface ITool {
	function: IHistoryFunction;
	type: IToolType;
	[property: string]: any;
}
