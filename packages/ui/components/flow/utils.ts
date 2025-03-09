import { IVariableType } from "../../lib/schema/flow/variable";

export function typeToColor(type: IVariableType): string {
	switch (type) {
		case "Execution":
			return "hsl(var(--foreground))";
		case "String":
			return "var(--pink-400)";
		case "Integer":
			return "var(--blue-500)";
		case "Float":
			return "var(--lime-400)";
		case "Boolean":
			return "var(--red-600)";
		case "Date":
			return "var(--amber-500)";
		case "PathBuf":
			return "var(--cyan-500)";
		case "Generic":
			return "var(--stone-500)";
		case "Struct":
			return "var(--indigo-500)";
		case "Byte":
			return "var(--teal-400)";
	}

	return "LightSkyBlue";
}
