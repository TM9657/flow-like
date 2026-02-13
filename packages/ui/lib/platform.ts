/**
 * Platform detection utilities
 */

/**
 * Detect Tauri environment (WebView)
 * Checks for common Tauri globals:
 * - __TAURI__ in Tauri v1
 * - __TAURI_INTERNALS__ / __TAURI_IPC__ in some builds
 */
export const isTauri = (): boolean => {
	if (typeof window === "undefined") return false;
	const w = window as any;
	return !!(w.__TAURI__ || w.__TAURI_IPC__ || w.__TAURI_INTERNALS__);
};
