import { IValueType, IVariableType } from "./schema/flow/variable";
import { convertJsonToUint8Array } from "./uint8";

export function defaultValueFromType(
	valueType: IValueType,
	variableType: IVariableType,
) {
	if (valueType === IValueType.Array) return [];
	if (valueType === IValueType.HashSet) return [];
	if (valueType === IValueType.HashMap) return {};
	switch (variableType) {
		case IVariableType.Boolean:
			return false;
		case IVariableType.Byte:
			return 0;
		case IVariableType.Date:
			return new Date().toISOString();
		case IVariableType.Float:
			return 0.0;
		case IVariableType.Integer:
			return 0;
		case IVariableType.PathBuf:
			return "";
		case IVariableType.String:
			return "";
		case IVariableType.Struct:
			return {};
		default:
			return null;
	}
}

export function encodedTypeDefault(
	valueType: IValueType,
	variableType: IVariableType,
): number[] | null {
	const value = defaultValueFromType(valueType, variableType);
	if (value === null) return null;
	return convertJsonToUint8Array(value) ?? null;
}
