export interface IPin {
    connected_to:   string[];
    data_type:      IVariableType;
    default_value?: number[] | null;
    depends_on:     string[];
    description:    string;
    friendly_name:  string;
    id:             string;
    index:          number;
    name:           string;
    pin_type:       IPinType;
    schema?:        null | string;
    valid_values?:  string[] | null;
    value_type:     IValueType;
    [property: string]: any;
}

export enum IVariableType {
    Boolean = "Boolean",
    Date = "Date",
    Execution = "Execution",
    Float = "Float",
    Generic = "Generic",
    Integer = "Integer",
    PathBuf = "PathBuf",
    String = "String",
    Struct = "Struct",
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
