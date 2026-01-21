"use client";

import { useCallback, useEffect, useRef } from "react";
import { uiElementValues as elementValues } from "../../../db/ui-state-db";

/**
 * Hook for storing and retrieving element values from global IndexedDB storage.
 * This allows element values to be accessible across pages and dialogs within an app.
 */
export function useElementStorage(appId: string | undefined) {
	const pendingUpdates = useRef<
		Map<string, { value: unknown; timeout: NodeJS.Timeout }>
	>(new Map());

	// Store an element value (debounced to avoid excessive writes)
	const storeElementValue = useCallback(
		(elementId: string, value: unknown) => {
			if (!appId) return;

			// Cancel any pending update for this element
			const pending = pendingUpdates.current.get(elementId);
			if (pending) {
				clearTimeout(pending.timeout);
			}

			// Debounce the write by 300ms
			const timeout = setTimeout(() => {
				elementValues.set(appId, elementId, value).catch((err) => {
					console.error("[ElementStorage] Failed to store element value:", err);
				});
				pendingUpdates.current.delete(elementId);
			}, 300);

			pendingUpdates.current.set(elementId, { value, timeout });
		},
		[appId],
	);

	// Get a single element value
	const getElementValue = useCallback(
		async <T>(elementId: string): Promise<T | null> => {
			if (!appId) return null;
			const value = await elementValues.getValue<T>(appId, elementId);
			return value ?? null;
		},
		[appId],
	);

	// Get all element values for the app
	const getAllElementValues = useCallback(async (): Promise<
		Record<string, unknown>
	> => {
		if (!appId) return {};
		return elementValues.getAllValues(appId);
	}, [appId]);

	// Clear all pending updates on unmount
	useEffect(() => {
		return () => {
			for (const pending of pendingUpdates.current.values()) {
				clearTimeout(pending.timeout);
			}
			pendingUpdates.current.clear();
		};
	}, []);

	return {
		storeElementValue,
		getElementValue,
		getAllElementValues,
	};
}
