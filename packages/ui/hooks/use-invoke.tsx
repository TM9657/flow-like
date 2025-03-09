"use client";
import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export function useInvoke<T>(
	path: string,
	args: any,
	deps: string[] = [],
	enabled = true,
): UseQueryResult<T, any> {
	const query = useQuery({
		queryKey: [...path.split("_"), ...deps],
		queryFn: async () => {
			try {
				const response = await invoke(path, args);
				return response as T;
			} catch (error) {
				console.error(JSON.stringify(error));
				throw error;
			}
		},
		enabled: enabled,
	});

	return query;
}
