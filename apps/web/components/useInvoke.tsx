"use client";

import type { UseQueryResult } from "@tanstack/react-query";
import { useQuery } from "@tanstack/react-query";

// Web-compatible useInvoke hook stub
// This replaces the Tauri-specific invoke functionality with empty stubs
export function useTauriInvoke<T>(
	command: string,
	args: any,
	deps: string[] = [],
	enabled = false,
): UseQueryResult<T, Error> {
	return useQuery<T, Error>({
		queryKey: [...command.split("_"), ...deps],
		queryFn: async (): Promise<T> => {
			console.warn(
				`Tauri invoke called in web context: ${command}. This is a stub and will return empty data.`,
			);
			return {} as T;
		},
		enabled: enabled,
	}) as UseQueryResult<T, Error>;
}
