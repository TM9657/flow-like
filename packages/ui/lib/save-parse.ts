import type { IVariable } from "../lib/schema/flow/variable";
import { parseUint8ArrayToJson } from "./uint8";

export function saveParseFloat(variable: IVariable, input: string): number {
	try {
		const parsedValue = Number.parseFloat(input);
		if (Number.isNaN(parsedValue)) {
			return parseUint8ArrayToJson(variable.default_value);
		}
		return parsedValue;
	} catch (error) {
		return parseUint8ArrayToJson(variable.default_value);
	}
}

export function saveParseInt(variable: IVariable, input: string): number {
	try {
		const parsedValue = Number.parseInt(input);
		if (Number.isNaN(parsedValue)) {
			return parseUint8ArrayToJson(variable.default_value);
		}
		return parsedValue;
	} catch (error) {
		return parseUint8ArrayToJson(variable.default_value);
	}
}
