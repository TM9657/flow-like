import type { LucideIcon } from "lucide-react";
import type React from "react";
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export type SpotlightItemType =
	| "navigation"
	| "project"
	| "action"
	| "dynamic"
	| "recent"
	| "shortcut"
	| "ai";

export type SpotlightMode = "search" | "flowpilot" | "quick-create";

export interface SpotlightItem {
	id: string;
	type: SpotlightItemType;
	label: string;
	description?: string;
	icon?: LucideIcon | React.ReactNode;
	iconUrl?: string;
	keywords?: string[];
	shortcut?: string;
	group?: string;
	action: () => void | Promise<void>;
	disabled?: boolean;
	priority?: number;
	subItems?: SpotlightItem[];
	/** If true, the spotlight dialog stays open after this action is executed */
	keepOpen?: boolean;
}

export interface SpotlightGroup {
	id: string;
	label: string;
	priority?: number;
}

interface FrecencyData {
	count: number;
	lastUsed: number;
	score: number;
}

interface SpotlightPersisted {
	recentItems: string[];
	frecencyData: Record<string, FrecencyData>;
}

interface SpotlightState extends SpotlightPersisted {
	isOpen: boolean;
	searchQuery: string;
	mode: SpotlightMode;
	staticItems: SpotlightItem[];
	dynamicItems: Map<string, SpotlightItem[]>;
	groups: SpotlightGroup[];
	selectedIndex: number;
	expandedProjectId: string | null;

	open: () => void;
	close: () => void;
	toggle: () => void;
	setSearchQuery: (query: string) => void;
	setSelectedIndex: (index: number) => void;
	setExpandedProject: (projectId: string | null) => void;
	setMode: (mode: SpotlightMode) => void;

	registerStaticItems: (items: SpotlightItem[]) => void;
	unregisterStaticItems: (itemIds: string[]) => void;

	registerDynamicItems: (sourceId: string, items: SpotlightItem[]) => void;
	unregisterDynamicItems: (sourceId: string) => void;

	registerGroup: (group: SpotlightGroup) => void;
	unregisterGroup: (groupId: string) => void;

	recordItemUsage: (itemId: string) => void;
	clearRecentItems: () => void;

	getAllItems: () => SpotlightItem[];
	getItemFrecency: (itemId: string) => number;
}

const MAX_RECENT_ITEMS = 8;
const FRECENCY_DECAY = 0.95;
const FRECENCY_BOOST = 10;

function calculateFrecencyScore(data: FrecencyData): number {
	const hoursSinceUse = (Date.now() - data.lastUsed) / (1000 * 60 * 60);
	const recencyBoost = Math.exp(-hoursSinceUse / 24);
	return data.count * FRECENCY_BOOST * recencyBoost;
}

export const useSpotlightStore = create<SpotlightState>()(
	persist(
		(set, get) => ({
			isOpen: false,
			searchQuery: "",
			mode: "search" as SpotlightMode,
			staticItems: [],
			dynamicItems: new Map(),
			recentItems: [],
			frecencyData: {},
			groups: [],
			selectedIndex: 0,
			expandedProjectId: null,

			open: () => set({ isOpen: true, selectedIndex: 0, mode: "search" }),
			close: () =>
				set({
					isOpen: false,
					searchQuery: "",
					selectedIndex: 0,
					expandedProjectId: null,
					mode: "search",
				}),
			toggle: () => {
				const current = get().isOpen;
				set({
					isOpen: !current,
					searchQuery: current ? "" : get().searchQuery,
					selectedIndex: 0,
					expandedProjectId: current ? null : get().expandedProjectId,
					mode: current ? "search" : get().mode,
				});
			},

			setSearchQuery: (query) => set({ searchQuery: query, selectedIndex: 0 }),
			setSelectedIndex: (index) => set({ selectedIndex: index }),
			setExpandedProject: (projectId) => set({ expandedProjectId: projectId }),
			setMode: (mode) => set({ mode, searchQuery: "" }),

			registerStaticItems: (items) =>
				set((state) => {
					const existingIds = new Set(state.staticItems.map((i) => i.id));
					const newItems = items.filter((item) => !existingIds.has(item.id));
					const updatedItems = state.staticItems.map((existing) => {
						const updated = items.find((i) => i.id === existing.id);
						return updated || existing;
					});
					return { staticItems: [...updatedItems, ...newItems] };
				}),

			unregisterStaticItems: (itemIds) =>
				set((state) => ({
					staticItems: state.staticItems.filter(
						(item) => !itemIds.includes(item.id),
					),
				})),

			registerDynamicItems: (sourceId, items) =>
				set((state) => {
					const newDynamicItems = new Map(state.dynamicItems);
					newDynamicItems.set(sourceId, items);
					return { dynamicItems: newDynamicItems };
				}),

			unregisterDynamicItems: (sourceId) =>
				set((state) => {
					const newDynamicItems = new Map(state.dynamicItems);
					newDynamicItems.delete(sourceId);
					return { dynamicItems: newDynamicItems };
				}),

			registerGroup: (group) =>
				set((state) => {
					const exists = state.groups.some((g) => g.id === group.id);
					if (exists) {
						return {
							groups: state.groups.map((g) => (g.id === group.id ? group : g)),
						};
					}
					return { groups: [...state.groups, group] };
				}),

			unregisterGroup: (groupId) =>
				set((state) => ({
					groups: state.groups.filter((g) => g.id !== groupId),
				})),

			recordItemUsage: (itemId) =>
				set((state) => {
					const now = Date.now();
					const existingData = state.frecencyData[itemId] || {
						count: 0,
						lastUsed: now,
						score: 0,
					};

					const newFrecencyData = {
						...state.frecencyData,
						[itemId]: {
							count: existingData.count + 1,
							lastUsed: now,
							score: calculateFrecencyScore({
								count: existingData.count + 1,
								lastUsed: now,
								score: 0,
							}),
						},
					};

					const filtered = state.recentItems.filter((id) => id !== itemId);
					const newRecent = [itemId, ...filtered].slice(0, MAX_RECENT_ITEMS);

					return {
						recentItems: newRecent,
						frecencyData: newFrecencyData,
					};
				}),

			clearRecentItems: () => set({ recentItems: [], frecencyData: {} }),

			getAllItems: () => {
				const state = get();
				const allDynamic = Array.from(state.dynamicItems.values()).flat();
				const allItems = [...state.staticItems, ...allDynamic];

				return allItems.sort((a, b) => {
					const frecencyA = state.frecencyData[a.id]?.score ?? 0;
					const frecencyB = state.frecencyData[b.id]?.score ?? 0;
					const priorityA = (a.priority ?? 0) + frecencyA;
					const priorityB = (b.priority ?? 0) + frecencyB;
					return priorityB - priorityA;
				});
			},

			getItemFrecency: (itemId) => {
				const state = get();
				return state.frecencyData[itemId]?.score ?? 0;
			},
		}),
		{
			name: "spotlight-storage",
			storage: createJSONStorage(() => localStorage),
			partialize: (state) => ({
				recentItems: state.recentItems,
				frecencyData: state.frecencyData,
			}),
		},
	),
);

export function useSpotlight() {
	return useSpotlightStore();
}

export function useSpotlightOpen() {
	return useSpotlightStore((state) => state.isOpen);
}

export function useSpotlightActions() {
	const store = useSpotlightStore();
	return {
		open: store.open,
		close: store.close,
		toggle: store.toggle,
		registerStaticItems: store.registerStaticItems,
		unregisterStaticItems: store.unregisterStaticItems,
		registerDynamicItems: store.registerDynamicItems,
		unregisterDynamicItems: store.unregisterDynamicItems,
		registerGroup: store.registerGroup,
		unregisterGroup: store.unregisterGroup,
		recordItemUsage: store.recordItemUsage,
	};
}
