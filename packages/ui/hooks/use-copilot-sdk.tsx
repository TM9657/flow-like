"use client";

import { useCallback, useEffect, useState } from "react";
import type {
	CopilotAuthStatus,
	CopilotConnectionConfig,
	CopilotModel,
} from "../components/flowpilot/types";
import { isTauri } from "../lib/platform";

interface UseCopilotSDKResult {
	/** Whether the Copilot SDK client is running */
	isRunning: boolean;
	/** Whether currently starting/stopping */
	isConnecting: boolean;
	/** Available Copilot models */
	models: CopilotModel[];
	/** Current auth status */
	authStatus: CopilotAuthStatus | null;
	/** Error message if any */
	error: string | null;
	/** Start the Copilot SDK client */
	start: (config?: CopilotConnectionConfig) => Promise<void>;
	/** Stop the Copilot SDK client */
	stop: () => Promise<void>;
	/** Refresh models list */
	refreshModels: () => Promise<void>;
	/** Refresh auth status */
	refreshAuthStatus: () => Promise<void>;
}

/**
 * Hook for managing GitHub Copilot SDK connection and state.
 * Only works in Tauri environment - returns a disabled state for web.
 */
export function useCopilotSDK(): UseCopilotSDKResult {
	const [isRunning, setIsRunning] = useState(false);
	const [isConnecting, setIsConnecting] = useState(false);
	const [models, setModels] = useState<CopilotModel[]>([]);
	const [authStatus, setAuthStatus] = useState<CopilotAuthStatus | null>(null);
	const [error, setError] = useState<string | null>(null);

	const isTauriEnv = isTauri();

	const start = useCallback(
		async (config?: CopilotConnectionConfig) => {
			if (!isTauriEnv) {
				setError("Copilot SDK is only available in desktop app");
				return;
			}

			setIsConnecting(true);
			setError(null);

			try {
				const { invoke } = await import("@tauri-apps/api/core");
				await invoke("copilot_sdk_start", {
					useStdio: config?.useStdio ?? true,
					cliUrl: config?.serverUrl,
				});
				setIsRunning(true);
			} catch (e) {
				const errMsg = e instanceof Error ? e.message : String(e);
				setError(errMsg);
				throw e;
			} finally {
				setIsConnecting(false);
			}
		},
		[isTauriEnv],
	);

	const stop = useCallback(async () => {
		if (!isTauriEnv) return;

		setIsConnecting(true);
		setError(null);

		try {
			const { invoke } = await import("@tauri-apps/api/core");
			await invoke("copilot_sdk_stop");
			setIsRunning(false);
			setModels([]);
			setAuthStatus(null);
		} catch (e) {
			const errMsg = e instanceof Error ? e.message : String(e);
			setError(errMsg);
			throw e;
		} finally {
			setIsConnecting(false);
		}
	}, [isTauriEnv]);

	const refreshModels = useCallback(async () => {
		if (!isTauriEnv || !isRunning) return;

		try {
			const { invoke } = await import("@tauri-apps/api/core");
			const result = await invoke<CopilotModel[]>("copilot_sdk_list_models");
			setModels(result);
		} catch (e) {
			const errMsg = e instanceof Error ? e.message : String(e);
			setError(errMsg);
		}
	}, [isTauriEnv, isRunning]);

	const refreshAuthStatus = useCallback(async () => {
		if (!isTauriEnv || !isRunning) return;

		try {
			const { invoke } = await import("@tauri-apps/api/core");
			const result = await invoke<CopilotAuthStatus>(
				"copilot_sdk_get_auth_status",
			);
			setAuthStatus(result);
		} catch (e) {
			const errMsg = e instanceof Error ? e.message : String(e);
			setError(errMsg);
		}
	}, [isTauriEnv, isRunning]);

	// Check initial running state
	useEffect(() => {
		if (!isTauriEnv) return;

		const checkRunning = async () => {
			try {
				const { invoke } = await import("@tauri-apps/api/core");
				const running = await invoke<boolean>("copilot_sdk_is_running");
				setIsRunning(running);
			} catch {
				// Ignore errors during initial check
			}
		};

		checkRunning();
	}, [isTauriEnv]);

	// Auto-fetch models and auth when running
	useEffect(() => {
		if (isRunning) {
			refreshModels();
			refreshAuthStatus();
		}
	}, [isRunning, refreshModels, refreshAuthStatus]);

	return {
		isRunning,
		isConnecting,
		models,
		authStatus,
		error,
		start,
		stop,
		refreshModels,
		refreshAuthStatus,
	};
}
