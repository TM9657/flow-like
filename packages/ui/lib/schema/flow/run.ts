export interface IRun {
	board: IBoard;
	end: ISystemTime;
	id: string;
	log_level: ILogLevel;
	start: ISystemTime;
	status: IRunStatus;
	traces: ITrace[];
	[property: string]: any;
}

export interface IBoard {
	comments: { [key: string]: IComment };
	created_at: ISystemTime;
	description: string;
	id: string;
	log_level: ILogLevel;
	name: string;
	nodes: { [key: string]: INode };
	refs: { [key: string]: string };
	stage: IExecutionStage;
	updated_at: ISystemTime;
	variables: { [key: string]: IVariable };
	version: number[];
	viewport: number[];
	[property: string]: any;
}

export interface IComment {
	author?: null | string;
	comment_type: ICommentType;
	content: string;
	coordinates: number[];
	id: string;
	timestamp: ISystemTime;
	[property: string]: any;
}

export enum ICommentType {
	Image = "Image",
	Text = "Text",
	Video = "Video",
}

export interface ISystemTime {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}

export enum ILogLevel {
	Debug = "Debug",
	Error = "Error",
	Fatal = "Fatal",
	Info = "Info",
	Warn = "Warn",
}

export interface INode {
	category: string;
	comment?: null | string;
	coordinates?: number[] | null;
	description: string;
	docs?: null | string;
	error?: null | string;
	friendly_name: string;
	icon?: null | string;
	id: string;
	long_running?: boolean | null;
	name: string;
	pins: { [key: string]: IPin };
	scores?: null | INodeScores;
	start?: boolean | null;
	[property: string]: any;
}

export interface IPin {
	connected_to: string[];
	data_type: IVariableType;
	default_value?: number[] | null;
	depends_on: string[];
	description: string;
	friendly_name: string;
	id: string;
	index: number;
	name: string;
	options?: null | IPinOptions;
	pin_type: IPinType;
	schema?: null | string;
	valid_values?: string[] | null;
	value_type: IValueType;
	[property: string]: any;
}

export enum IVariableType {
	Boolean = "Boolean",
	Byte = "Byte",
	Date = "Date",
	Execution = "Execution",
	Float = "Float",
	Generic = "Generic",
	Integer = "Integer",
	PathBuf = "PathBuf",
	String = "String",
	Struct = "Struct",
}

export interface IPinOptions {
	enforce_generic_value_type?: boolean | null;
	enforce_schema?: boolean | null;
	range?: number[] | null;
	step?: number | null;
	valid_values?: string[] | null;
	[property: string]: any;
}

export enum IPinType {
	Input = "Input",
	Output = "Output",
}

export enum IValueType {
	Array = "Array",
	HashMap = "HashMap",
	HashSet = "HashSet",
	Normal = "Normal",
}

/**
 * Represents quality metrics for a node, with scores ranging from 0 to 10. Higher scores
 * indicate worse performance in each category.
 *
 * # Score Categories * `privacy` - Measures data protection and confidentiality level *
 * `security` - Assesses resistance against potential attacks * `performance` - Evaluates
 * computational efficiency and speed * `governance` - Indicates compliance with policies
 * and regulations
 */
export interface INodeScores {
	governance: number;
	performance: number;
	privacy: number;
	security: number;
	[property: string]: any;
}

export enum IExecutionStage {
	Dev = "Dev",
	Int = "Int",
	PreProd = "PreProd",
	Prod = "Prod",
	QA = "QA",
}

export interface IVariable {
	category?: null | string;
	data_type: IVariableType;
	default_value?: number[] | null;
	description?: null | string;
	editable: boolean;
	exposed: boolean;
	id: string;
	name: string;
	secret: boolean;
	value_type: IValueType;
	[property: string]: any;
}

export enum IRunStatus {
	Failed = "Failed",
	Running = "Running",
	Stopped = "Stopped",
	Success = "Success",
}

export interface ITrace {
	end: ISystemTime;
	id: string;
	logs: ILogMessage[];
	node_id: string;
	start: ISystemTime;
	variables?: { [key: string]: IVariable } | null;
	[property: string]: any;
}

export interface ILogMessage {
	end: ISystemTime;
	log_level: ILogLevel;
	message: string;
	operation_id?: null | string;
	start: ISystemTime;
	stats?: null | ILogStat;
	[property: string]: any;
}

export interface ILogStat {
	bit_ids?: string[] | null;
	token_in?: number | null;
	token_out?: number | null;
	[property: string]: any;
}
