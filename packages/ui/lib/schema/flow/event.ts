export interface IEvent {
	active: boolean;
	board_id: string;
	board_version?: any[] | null;
	canary?: null | ICanaryObject;
	config: number[];
	created_at: ICreatedAt;
	description: string;
	event_type: string;
	event_version: any[];
	id: string;
	name: string;
	node_id: string;
	notes?: INotesClass | null;
	priority: number;
	updated_at: ICreatedAt;
	variables: { [key: string]: IVariableValue };
	[property: string]: any;
}

export interface ICanaryObject {
	board_id: string;
	board_version?: any[] | null;
	created_at: ICreatedAt;
	node_id: string;
	updated_at: ICreatedAt;
	variables: { [key: string]: IVariableValue };
	weight: number;
	[property: string]: any;
}

export interface ICreatedAt {
	nanos_since_epoch: number;
	secs_since_epoch: number;
	[property: string]: any;
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

export enum IValueType {
	Array = "Array",
	HashMap = "HashMap",
	HashSet = "HashSet",
	Normal = "Normal",
}

export interface INotesClass {
	NOTES?: string;
	URL?: string;
}
