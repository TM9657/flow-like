"use client";
import {
	type UseQueryResult,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";

type BackendFunction<T, Args extends any[]> = (...args: Args) => Promise<T>;

/**
 * Custom hook to invoke an asynchronous backend function using React Query.
 * Handles functions with multiple arguments passed as an array.
 *
 * @template T The expected return type of the backend function.
 * @template Args An array type representing the arguments of the backend function.
 * @param {BackendFunction<T, Args>} backendFn The asynchronous function to call.
 * @param {Args} args An array containing the arguments to pass to the backend function.
 * @param {boolean} [enabled=true] Whether the query should be enabled and run automatically.
 * Defaults to true.
 * @param {any[]} [additionalDeps=[]] Optional additional dependencies to include in the queryKey,
 * beyond the function name and arguments. Useful if the query
 * depends on external factors not passed as arguments.
 * @returns {UseQueryResult<T, Error>} The result object from React Query, containing data, error, status, etc.
 */
export function useInvoke<T, Args extends any[]>(
	backendFn: BackendFunction<T, Args>,
	args: Args,
	enabled = true,
	additionalDeps: any[] = [],
): UseQueryResult<T, Error> {
	const query = useQuery<T, Error>({
		queryKey: [backendFn.name || "backendFn", ...args, ...additionalDeps],
		queryFn: async () => {
			try {
				const response = await backendFn(...args);
				return response; // No need to cast if types are correctly inferred/set
			} catch (error) {
				console.error("Error invoking backend function:", error);
				if (error instanceof Error) {
					throw error;
				} else {
					throw new Error(String(error));
				}
			}
		},
		enabled,
	});

	return query;
}

/**
 * Custom hook that returns a function to invalidate React Query cache entries
 * associated with a specific backend function used via `useInvoke`.
 * This typically invalidates all queries starting with the function's name key prefix.
 *
 * @returns {function(backendFn: BackendFunction<any, any[]>): Promise<void>}
 * A function that accepts the backend function whose queries should be invalidated.
 */
export function useInvalidateInvoke() {
	const queryClient = useQueryClient();

	/**
	 * Invalidates queries associated with the given backend function.
	 * Uses prefix matching based on the function name.
	 *
	 * @template T The return type of the backend function (often not needed for invalidation).
	 * @template Args The arguments array type of the backend function (often not needed for invalidation).
	 * @param {BackendFunction<T, Args>} backendFn The backend function used in `useInvoke` calls.
	 * @returns {Promise<void>} A promise that resolves when the invalidation is complete.
	 */
	const invalidate = <T, Args extends any[]>(
		backendFn: BackendFunction<T, Args>,
		args: Args,
		additionalDeps: any[] = [],
	): Promise<void> => {
		const queryKeyPrefix = [
			backendFn.name || "backendFn",
			...args,
			...additionalDeps,
		];
		console.log(`Invalidating queries with prefix:`, queryKeyPrefix);
		return queryClient.invalidateQueries({
			queryKey: queryKeyPrefix,
		});
	};

	return invalidate;
}
