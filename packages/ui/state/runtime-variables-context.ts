import { createContext, useContext } from "react";

export interface RuntimeVariableValue {
	variableId: string;
	value: number[];
}

/**
 * A stored device value with the denormalized fields the row carries alongside
 * it. They are what makes the surface survive a board read failure: the name,
 * the owning board and the secret flag are already here, so a user whose role
 * cannot read the flows can still see and edit what this app asked them for.
 */
export interface StoredRuntimeVariable extends RuntimeVariableValue {
	boardId: string;
	variableName: string;
	isSecret: boolean;
	updatedAt: string;
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

	/**
	 * Every stored row for an app, with its denormalized metadata. Used by the
	 * Setup surface for `updatedAt`, for the degraded view when the board read
	 * fails, and to find rows whose variable no longer exists.
	 */
	listValues: (appId: string) => Promise<StoredRuntimeVariable[]>;

	/**
	 * Forget one stored value. The variable falls back to whatever the flow
	 * defines.
	 */
	deleteValue: (appId: string, variableId: string) => Promise<void>;

	/**
	 * Forget several stored values at once — the orphan sweep. Passing no ids
	 * clears every value this app holds on the device.
	 */
	deleteValues: (appId: string, variableIds?: string[]) => Promise<void>;

	/**
	 * Observe the stored rows for an app. The callback fires once on
	 * subscription and again on every write, so a surface does not have to
	 * re-read after its own save. Returns an unsubscribe function.
	 */
	subscribe: (
		appId: string,
		onChange: (values: StoredRuntimeVariable[]) => void,
	) => () => void;
}

const RuntimeVariablesContext = createContext<
	RuntimeVariablesContextValue | undefined
>(undefined);

export const RuntimeVariablesProvider = RuntimeVariablesContext.Provider;

export function useRuntimeVariables():
	| RuntimeVariablesContextValue
	| undefined {
	return useContext(RuntimeVariablesContext);
}
