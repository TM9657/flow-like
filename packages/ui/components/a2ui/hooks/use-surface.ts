"use client";

import { useCallback, useMemo } from "react";
import { useSurfaceManager } from "../SurfaceManager";
import type { A2UIClientMessage } from "../types";

export interface UseSurfaceOptions {
	onMessage?: (message: A2UIClientMessage) => void;
}

export function useSurface(surfaceId: string, options: UseSurfaceOptions = {}) {
	const { surfaces, handleServerMessage, getSurface } = useSurfaceManager();

	const surface = useMemo(() => getSurface(surfaceId), [getSurface, surfaceId]);

	const deleteSurface = useCallback(() => {
		handleServerMessage({
			type: "deleteSurface",
			surfaceId,
		});
	}, [handleServerMessage, surfaceId]);

	const sendAction = useCallback(
		(name: string, context: Record<string, unknown> = {}) => {
			options.onMessage?.({
				type: "userAction",
				name,
				surfaceId,
				sourceComponentId: "",
				timestamp: Date.now(),
				context,
			});
		},
		[options, surfaceId],
	);

	return {
		surface,
		deleteSurface,
		sendAction,
		exists: !!surface,
	};
}

export function useSurfaceComponent(surfaceId: string, componentId: string) {
	const { getSurface } = useSurfaceManager();

	const surface = useMemo(() => getSurface(surfaceId), [getSurface, surfaceId]);

	return useMemo(
		() => surface?.components?.[componentId],
		[surface?.components, componentId],
	);
}
