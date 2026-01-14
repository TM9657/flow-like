"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { useBackend } from "../state/backend-state";
import type { IWidget, Version } from "../state/backend-state/widget-state";
import type { WidgetRef } from "../components/a2ui/types";

export interface ResolvedWidget {
	widget: IWidget;
	ref: WidgetRef;
	resolvedAt: number;
}

export interface UseWidgetResolverOptions {
	cacheTimeout?: number;
}

export interface UseWidgetResolverResult {
	resolveWidget: (ref: WidgetRef) => Promise<IWidget | null>;
	resolveWidgets: (refs: WidgetRef[]) => Promise<Map<string, IWidget>>;
	getFromCache: (ref: WidgetRef) => IWidget | undefined;
	clearCache: () => void;
	isResolving: boolean;
	cache: Map<string, ResolvedWidget>;
}

function getRefKey(ref: WidgetRef): string {
	return `${ref.appId}:${ref.widgetId}${ref.version ? `:${ref.version}` : ""}`;
}

function parseVersion(versionStr?: string): Version | undefined {
	if (!versionStr) return undefined;
	const parts = versionStr.split(".").map(Number);
	if (parts.length !== 3 || parts.some(Number.isNaN)) return undefined;
	return [parts[0], parts[1], parts[2]] as Version;
}

export function useWidgetResolver(
	options: UseWidgetResolverOptions = {},
): UseWidgetResolverResult {
	const { cacheTimeout = 5 * 60 * 1000 } = options;
	const backend = useBackend();
	const [isResolving, setIsResolving] = useState(false);
	const cacheRef = useRef<Map<string, ResolvedWidget>>(new Map());

	const isExpired = useCallback(
		(resolvedWidget: ResolvedWidget): boolean => {
			return Date.now() - resolvedWidget.resolvedAt > cacheTimeout;
		},
		[cacheTimeout],
	);

	const getFromCache = useCallback(
		(ref: WidgetRef): IWidget | undefined => {
			const key = getRefKey(ref);
			const cached = cacheRef.current.get(key);
			if (cached && !isExpired(cached)) {
				return cached.widget;
			}
			if (cached && isExpired(cached)) {
				cacheRef.current.delete(key);
			}
			return undefined;
		},
		[isExpired],
	);

	const resolveWidget = useCallback(
		async (ref: WidgetRef): Promise<IWidget | null> => {
			const cached = getFromCache(ref);
			if (cached) return cached;

			setIsResolving(true);
			try {
				const version = parseVersion(ref.version);
				const widget = await backend.widgetState.getWidget(
					ref.appId,
					ref.widgetId,
					version,
				);

				const key = getRefKey(ref);
				cacheRef.current.set(key, {
					widget,
					ref,
					resolvedAt: Date.now(),
				});

				return widget;
			} catch (error) {
				console.error("Failed to resolve widget:", ref, error);
				return null;
			} finally {
				setIsResolving(false);
			}
		},
		[backend.widgetState, getFromCache],
	);

	const resolveWidgets = useCallback(
		async (refs: WidgetRef[]): Promise<Map<string, IWidget>> => {
			const results = new Map<string, IWidget>();
			const toFetch: WidgetRef[] = [];

			for (const ref of refs) {
				const cached = getFromCache(ref);
				if (cached) {
					results.set(getRefKey(ref), cached);
				} else {
					toFetch.push(ref);
				}
			}

			if (toFetch.length === 0) return results;

			setIsResolving(true);
			try {
				await Promise.all(
					toFetch.map(async (ref) => {
						try {
							const version = parseVersion(ref.version);
							const widget = await backend.widgetState.getWidget(
								ref.appId,
								ref.widgetId,
								version,
							);

							const key = getRefKey(ref);
							cacheRef.current.set(key, {
								widget,
								ref,
								resolvedAt: Date.now(),
							});
							results.set(key, widget);
						} catch (error) {
							console.error("Failed to resolve widget:", ref, error);
						}
					}),
				);
			} finally {
				setIsResolving(false);
			}

			return results;
		},
		[backend.widgetState, getFromCache],
	);

	const clearCache = useCallback(() => {
		cacheRef.current.clear();
	}, []);

	const cache = useMemo(() => cacheRef.current, []);

	return {
		resolveWidget,
		resolveWidgets,
		getFromCache,
		clearCache,
		isResolving,
		cache,
	};
}
