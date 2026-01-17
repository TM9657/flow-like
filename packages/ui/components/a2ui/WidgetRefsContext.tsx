"use client";

import {
	type ReactNode,
	createContext,
	useCallback,
	useContext,
	useMemo,
} from "react";
import type { IWidgetRef } from "../../state/backend-state/page-state";

export type { IWidgetRef };

export interface WidgetRefsContextValue {
	widgetRefs: Map<string, IWidgetRef> | Record<string, IWidgetRef>;
	getWidgetRef: (instanceId: string) => IWidgetRef | undefined;
}

const WidgetRefsContext = createContext<WidgetRefsContextValue | null>(null);

export function useWidgetRefs() {
	return useContext(WidgetRefsContext);
}

export interface WidgetRefsProviderProps {
	widgetRefs?: Map<string, IWidgetRef> | Record<string, IWidgetRef>;
	children: ReactNode;
}

export function WidgetRefsProvider({
	widgetRefs,
	children,
}: WidgetRefsProviderProps) {
	const getWidgetRef = useCallback(
		(instanceId: string): IWidgetRef | undefined => {
			if (!widgetRefs) return undefined;
			if (widgetRefs instanceof Map) {
				return widgetRefs.get(instanceId);
			}
			return widgetRefs[instanceId];
		},
		[widgetRefs],
	);

	const value = useMemo(
		() => ({ widgetRefs: widgetRefs ?? {}, getWidgetRef }),
		[widgetRefs, getWidgetRef],
	);

	return (
		<WidgetRefsContext.Provider value={value}>
			{children}
		</WidgetRefsContext.Provider>
	);
}
