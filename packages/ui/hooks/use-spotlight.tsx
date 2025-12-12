"use client";

import { useCallback, useEffect, useRef } from "react";
import {
	type SpotlightGroup,
	type SpotlightItem,
	useSpotlightStore,
} from "../state/spotlight-state";

const SPOTLIGHT_SHORTCUT_KEY = "k";

export function useSpotlightKeyboard() {
	const { open, close, isOpen, toggle } = useSpotlightStore();

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (
				event.key === SPOTLIGHT_SHORTCUT_KEY &&
				(event.metaKey || event.ctrlKey)
			) {
				event.preventDefault();
				event.stopPropagation();
				toggle();
			}

			if (event.key === "Escape" && isOpen) {
				event.preventDefault();
				close();
			}
		};

		window.addEventListener("keydown", handleKeyDown, true);
		return () => window.removeEventListener("keydown", handleKeyDown, true);
	}, [open, close, isOpen, toggle]);

	return { open, close, toggle, isOpen };
}

interface UseSpotlightItemsOptions {
	sourceId: string;
	items: SpotlightItem[];
	enabled?: boolean;
}

export function useSpotlightItems({
	sourceId,
	items,
	enabled = true,
}: UseSpotlightItemsOptions) {
	const { registerDynamicItems, unregisterDynamicItems } = useSpotlightStore();
	const itemsRef = useRef(items);

	useEffect(() => {
		itemsRef.current = items;
	}, [items]);

	useEffect(() => {
		if (enabled && items.length > 0) {
			registerDynamicItems(sourceId, items);
		}

		return () => {
			unregisterDynamicItems(sourceId);
		};
	}, [sourceId, items, enabled, registerDynamicItems, unregisterDynamicItems]);
}

interface UseSpotlightGroupOptions {
	group: SpotlightGroup;
	enabled?: boolean;
}

export function useSpotlightGroup({
	group,
	enabled = true,
}: UseSpotlightGroupOptions) {
	const { registerGroup, unregisterGroup } = useSpotlightStore();

	useEffect(() => {
		if (enabled) {
			registerGroup(group);
		}

		return () => {
			unregisterGroup(group.id);
		};
	}, [group, enabled, registerGroup, unregisterGroup]);
}

export function useSpotlightStaticItems(items: SpotlightItem[]) {
	const { registerStaticItems, unregisterStaticItems } = useSpotlightStore();
	const itemIdsRef = useRef<string[]>([]);

	useEffect(() => {
		const newIds = items.map((item) => item.id);
		registerStaticItems(items);
		itemIdsRef.current = newIds;

		return () => {
			unregisterStaticItems(itemIdsRef.current);
		};
	}, [items, registerStaticItems, unregisterStaticItems]);
}

export function useSpotlightAction(callback: () => void | Promise<void>) {
	const { close, recordItemUsage } = useSpotlightStore();

	return useCallback(
		async (itemId?: string) => {
			if (itemId) {
				recordItemUsage(itemId);
			}
			close();
			await callback();
		},
		[callback, close, recordItemUsage],
	);
}
