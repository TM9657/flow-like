import { createContext, useContext } from "react";
import type { IVariable } from "../lib/schema/flow/board";

export interface RuntimeVariableValue {
	variableId: string;
	value: number[];
}

export interface RuntimeVariablesContextValue {
	/**
	 * Get stored runtime variable values for an app
	 */
	getValues: (appId: string) => Promise<Map<string, RuntimeVariableValue>>;

	/**
	 * Save runtime variable values for an app
	 */
	saveValues: (
		appId: string,
		boardId: string,
		values: Array<{
			variableId: string;
			variableName: string;
			value: number[];
			isSecret: boolean;
		}>,
	) => Promise<void>;

	/**
	 * Check if all required runtime variables are configured
	 */
	hasAllValues: (appId: string, variableIds: string[]) => Promise<boolean>;
}

const RuntimeVariablesContext = createContext<
	RuntimeVariablesContextValue | undefined
>(undefined);

export const RuntimeVariablesProvider = RuntimeVariablesContext.Provider;

export function useRuntimeVariables(): RuntimeVariablesContextValue | undefined {
	return useContext(RuntimeVariablesContext);
}
