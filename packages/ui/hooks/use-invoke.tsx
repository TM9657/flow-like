"use client";
import { invoke } from "@tauri-apps/api/core";
import { useQuery, type UseQueryResult } from "@tanstack/react-query";

export function useInvoke<T>(
	path: string,
	args: any,
	deps: string[] = [],
	enabled: boolean = true,
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
