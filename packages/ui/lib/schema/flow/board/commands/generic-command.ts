export interface IGenericCommand {
	command_type: ICommandType;
	comment?: IComment;
	current_layer?: null | string;
	old_comment?: null | IComment;
	node?: INode;
	new_comments?: IComment[];
	new_layers?: ILayer[];
	new_nodes?: INode[];
	offset?: number[];
	old_mouse?: number[] | null;
	original_comments?: IComment[];
	original_layers?: ILayer[];
	original_nodes?: INode[];
	from_coordinates?: number[] | null;
	node_id?: string;
	to_coordinates?: number[];
	connected_nodes?: INode[];
	old_node?: null | INode;
	from_node?: string;
	from_pin?: string;
	to_node?: string;
	to_pin?: string;
	old_pin?: null | IPin;
	pin?: IPin;
	variable?: IVariable;
	old_variable?: null | IVariable;
	layer?: ILayer;
	node_ids?: string[];
	old_layer?: null | ILayer;
	child_layers?: string[];
	layer_nodes?: string[];
	layers?: ILayer[];
	nodes?: INode[];
	preserve_nodes?: boolean;
	[property: string]: any;
}

export enum ICommandType {
	AddNode = "AddNode",
	ConnectPin = "ConnectPin",
	CopyPaste = "CopyPaste",
	DisconnectPin = "DisconnectPin",
	MoveNode = "MoveNode",
	RemoveComment = "RemoveComment",
	RemoveLayer = "RemoveLayer",
	RemoveNode = "RemoveNode",
	RemoveVariable = "RemoveVariable",
	UpdateNode = "UpdateNode",
	UpsertComment = "UpsertComment",
	UpsertLayer = "UpsertLayer",
	UpsertPin = "UpsertPin",
	UpsertVariable = "UpsertVariable",
}

export interface IComment {
	author?: null | string;
	comment_type: ICommentType;
	content: string;
	coordinates: number[];
	height?: number | null;
	id: string;
	layer?: null | string;
	timestamp: ISystemTime;
	width?: number | null;
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

export interface INode {
	category: string;
	comment?: null | string;
	coordinates?: number[] | null;
	description: string;
	docs?: null | string;
	error?: null | string;
	event_callback?: boolean | null;
	friendly_name: string;
	icon?: null | string;
	id: string;
	layer?: null | string;
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

export interface ILayer {
	comment?: null | string;
	comments: { [key: string]: IComment };
	coordinates: number[];
	error?: null | string;
	id: string;
	name: string;
	nodes: { [key: string]: INode };
	parent_id?: null | string;
	pins: { [key: string]: IPin };
	type: ILayerType;
	variables: { [key: string]: IVariable };
	[property: string]: any;
}

export enum ILayerType {
	Collapsed = "Collapsed",
	Function = "Function",
	Macro = "Macro",
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
