import { type IVariable, IVariableType } from "../../../lib/schema/flow/variable";
import { BoolArrayVariable } from "./bool-array-variable";
import { BoolVariable } from "./bool-variable";
import { DateArrayVariable } from "./date-array-variable";
import { DateVariable } from "./date-variable";
import { StringArrayVariable } from "./string-array-variable";
import { StringVariable } from "./string-variable";
import { FloatVariable } from "./float-variable";
import { FloatArrayVariable } from "./float-array-variable";
import { IntegerVariable } from "./integer-variable";
import { IntegerArrayVariable } from "./integer-array-variable";
import { PathbufVariable } from "./pathbuf-variable";
import { IValueType } from "../../../lib/schema/flow/pin";

export function VariablesMenuEdit({ variable, updateVariable }: Readonly<{ variable: IVariable, updateVariable: (variable: IVariable) => Promise<void> }>) {
    if (variable.data_type === IVariableType.String && (variable.value_type === IValueType.Normal)) {
        return <StringVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.String && (variable.value_type === IValueType.Array)) {
        return <StringArrayVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Boolean && (variable.value_type === IValueType.Normal)) {
        return <BoolVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Boolean && (variable.value_type === IValueType.Array)) {
        return <BoolArrayVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Date && (variable.value_type === IValueType.Normal)) {
        return <DateVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Date && (variable.value_type === IValueType.Array)) {
        return <DateArrayVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Float && (variable.value_type === IValueType.Normal)) {
        return <FloatVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Float && (variable.value_type === IValueType.Array)) {
        return <FloatArrayVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Integer && (variable.value_type === IValueType.Normal)) {
        return <IntegerVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.Integer && (variable.value_type === IValueType.Array)) {
        return <IntegerArrayVariable variable={variable} onChange={updateVariable} />
    }

    if (variable.data_type === IVariableType.PathBuf && (variable.value_type === IValueType.Normal))  {
        return <PathbufVariable variable={variable} onChange={updateVariable} />
    }

    return null;
}