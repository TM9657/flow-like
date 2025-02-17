export interface IBoard {
    comments:    { [key: string]: IComment };
    created_at:  ISystemTime;
    description: string;
    id:          string;
    name:        string;
    nodes:       { [key: string]: INode };
    refs:        { [key: string]: string };
    stage:       IExecutionStage;
    updated_at:  ISystemTime;
    variables:   { [key: string]: IVariable };
    version:     number[];
    viewport:    number[];
    [property: string]: any;
}

export interface IComment {
    author?:      null | string;
    comment_type: ICommentType;
    content:      string;
    coordinates:  number[];
    id:           string;
    timestamp:    ISystemTime;
    [property: string]: any;
}

export enum ICommentType {
    Image = "Image",
    Text = "Text",
    Video = "Video",
}

export interface ISystemTime {
    nanos_since_epoch: number;
    secs_since_epoch:  number;
    [property: string]: any;
}

export interface INode {
    category:      string;
    comment?:      null | string;
    coordinates?:  number[] | null;
    description:   string;
    error?:        null | string;
    friendly_name: string;
    icon?:         null | string;
    id:            string;
    long_running?: boolean | null;
    name:          string;
    pins:          { [key: string]: IPin };
    scores?:       null | INodeScores;
    start?:        boolean | null;
    [property: string]: any;
}

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
    options?:       null | IPinOptions;
    pin_type:       IPinType;
    schema?:        null | string;
    valid_values?:  string[] | null;
    value_type:     IValueType;
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
    enforce_schema?: boolean | null;
    range?:          number[] | null;
    step?:           number | null;
    valid_values?:   string[] | null;
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
 * INode Scores. Each score ranges from 0 to 10. INode Scores. From 0 - 10 The higher the
 * score, the worse the node is in this category: - Privacy: Higher score means less
 * privacy. - Security: Higher score means less security. - Performance: Higher score means
 * worse performance. - Governance: Higher score means less compliance with governance. -
 * security: Assesses the node's resistance to attacks. - performance: Evaluates the node's
 * efficiency and speed. - governance: Indicates the node's compliance with policies and
 * regulations. The higher the score, the worse the node is in this category
 */
export interface INodeScores {
    governance:  number;
    performance: number;
    privacy:     number;
    security:    number;
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
    category?:      null | string;
    data_type:      IVariableType;
    default_value?: number[] | null;
    description?:   null | string;
    editable:       boolean;
    exposed:        boolean;
    id:             string;
    name:           string;
    secret:         boolean;
    value_type:     IValueType;
    [property: string]: any;
}
