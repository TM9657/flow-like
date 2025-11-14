export interface IRemoveLayer {
	child_layers: string[];
	layer: ILayer;
	layer_nodes: string[];
	layers: ILayer[];
	nodes: INodeValue[];
	preserve_nodes: boolean;
	[property: string]: any;
}

export interface ILayer {
	color?: null | string;
	comment?: null | string;
	comments: { [key: string]: ICommentValue };
	coordinates: any[];
	error?: null | string;
	hash?: number | null;
	id: string;
	in_coordinates?: any[] | null;
	name: string;
	nodes: { [key: string]: INodeValue };
	out_coordinates?: any[] | null;
	parent_id?: null | string;
	pins: { [key: string]: IPinValue };
	type: IType;
	variables: { [key: string]: IVariableValue };
	[property: string]: any;
}

export interface ICommentValue {
	author?: null | string;
	color?: null | string;
	comment_type: ICommentType;
	content: string;
	coordinates: any[];
	hash?: number | null;
	height?: number | null;
	id: string;
	is_locked?: boolean | null;
	layer?: null | string;
	timestamp: ITimestamp;
	width?: number | null;
	z_index?: number | null;
	[property: string]: any;
}

export enum ICommentType {
	Image = "Image",
	Text = "Text",
	Video = "Video",
}

export interface ITimestamp {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
}

export interface INodeValue {
	category: string;
	comment?: null | string;
	coordinates?: any[] | null;
	description: string;
	docs?: null | string;
	error?: null | string;
	event_callback?: boolean | null;
	fn_refs?: null | IFnRefsObject;
	friendly_name: string;
	hash?: number | null;
	icon?: null | string;
	id: string;
	layer?: null | string;
	long_running?: boolean | null;
	name: string;
	pins: { [key: string]: IPinValue };
	scores?: null | IScoresObject;
	start?: boolean | null;
	[property: string]: any;
}

export interface IFnRefsObject {
	can_be_referenced_by_fns: boolean;
	can_reference_fns: boolean;
	fn_refs: string[];
	[property: string]: any;
}

export interface IPinValue {
	connected_to: string[];
	data_type: IDataType;
	default_value?: number[] | null;
	depends_on: string[];
	description: string;
	friendly_name: string;
	id: string;
	index: number;
	name: string;
	options?: null | IOptionsObject;
	pin_type: IPinType;
	schema?: null | string;
	value_type: IValueType;
	[property: string]: any;
}

export enum IDataType {
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

export interface IOptionsObject {
	enforce_generic_value_type?: boolean | null;
	enforce_schema?: boolean | null;
	range?: any[] | null;
	sensitive?: boolean | null;
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
 * Represents quality metrics for a node, with scores ranging from 0 to 10.
 * Higher scores indicate worse performance in each category.
 *
 * # Score Categories
 * * `privacy` - Measures data protection and confidentiality level
 * * `security` - Assesses resistance against potential attacks
 * * `performance` - Evaluates computational efficiency and speed
 * * `governance` - Indicates compliance with policies and regulations
 */
export interface IScoresObject {
	governance: number;
	performance: number;
	privacy: number;
	security: number;
	[property: string]: any;
}

export enum IType {
	Collapsed = "Collapsed",
	Function = "Function",
	Macro = "Macro",
}

export interface IVariableValue {
	category?: null | string;
	data_type: IDataType;
	default_value?: number[] | null;
	description?: null | string;
	editable: boolean;
	exposed: boolean;
	hash?: number | null;
	id: string;
	name: string;
	secret: boolean;
	value_type: IValueType;
	[property: string]: any;
}
