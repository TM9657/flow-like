export function subscribeWidgetQueryRefresh(
	refresh: () => Promise<void>,
	windowTarget: Pick<
		Window,
		"addEventListener" | "removeEventListener"
	> = window,
	documentTarget: Pick<
		Document,
		"addEventListener" | "removeEventListener" | "visibilityState"
	> = document,
): () => void {
	let refreshTimer: ReturnType<typeof setTimeout> | undefined;
	const requestRefresh = () => {
		if (documentTarget.visibilityState !== "visible" || refreshTimer) return;
		// Native webviews may report focus and visibility changes together.
		// Coalesce that burst while allowing reconnect to restart a stalled fetch.
		refreshTimer = setTimeout(() => {
			refreshTimer = undefined;
			if (documentTarget.visibilityState !== "visible") return;
			void refresh().catch((error) => {
				console.warn(
					"[WidgetSync] Failed to refresh widgets on resume:",
					error,
				);
			});
		}, 50);
	};

	windowTarget.addEventListener("focus", requestRefresh);
	windowTarget.addEventListener("online", requestRefresh);
	documentTarget.addEventListener("visibilitychange", requestRefresh);
	return () => {
		clearTimeout(refreshTimer);
		windowTarget.removeEventListener("focus", requestRefresh);
		windowTarget.removeEventListener("online", requestRefresh);
		documentTarget.removeEventListener("visibilitychange", requestRefresh);
	};
}
