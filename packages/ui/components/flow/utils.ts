import type { IVariableType } from "../../lib/schema/flow/variable";

export function typeToColor(type: IVariableType): string {
	switch (type) {
		case "Execution":
			return "var(--foreground)";
		case "String":
			return "var(--pin-string)";
		case "Integer":
			return "var(--pin-integer)";
		case "Float":
			return "var(--pin-float)";
		case "Boolean":
			return "var(--pin-boolean)";
		case "Date":
			return "var(--pin-date)";
		case "PathBuf":
			return "var(--pin-pathbuf)";
		case "Generic":
			return "var(--pin-generic)";
		case "Struct":
			return "var(--pin-struct)";
		case "Geometry":
			return "var(--pin-geometry)";
		case "Byte":
			return "var(--pin-byte)";
	}

	return "var(--pin-byte)";
}
