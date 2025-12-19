import { create } from "zustand";
import type { ILogLevel, ILogMetadata } from "../lib";
import type { IBackendState } from "./backend-state";

export interface ILogAggregationFilter {
	appId: string;
	boardId: string;
	nodeId?: string;
	from?: number;
	to?: number;
	status?: ILogLevel;
	limit?: number;
	offset?: number;
	lastMeta?: ILogMetadata;
}

interface ILogAggregationState {
	currentLogs: ILogMetadata[];
	filter?: ILogAggregationFilter;
	currentMetadata?: ILogMetadata;
	refetchLogs: (backend: IBackendState) => Promise<void>;
	setFilter(
		backend: IBackendState,
		filter: ILogAggregationFilter,
	): Promise<void>;
	setCurrentMetadata: (meta?: ILogMetadata) => void;
}

export const useLogAggregation = create<ILogAggregationState>((set, get) => ({
	currentLogs: [],
	filter: undefined,
	currentMetadata: undefined,
	setFilter: async (backend: IBackendState, filter: ILogAggregationFilter) => {
		const currentFilter = get().filter;
		const boardChanged =
			currentFilter?.appId !== filter.appId ||
			currentFilter?.boardId !== filter.boardId;

		// Clear currentMetadata when board changes to avoid showing stale logs
		if (boardChanged) {
			set({ filter, currentMetadata: undefined });
		} else {
			set({ filter });
		}

		const runs = await backend.boardState.listRuns(
			filter.appId,
			filter.boardId,
			filter.nodeId,
			filter.from,
			filter.to,
			filter.status,
			filter.lastMeta,
			filter.offset,
			filter.limit,
		);

		set({ currentLogs: runs.toSorted((a, b) => b.start - a.start) });
	},
	setCurrentMetadata: (meta?: ILogMetadata) => {
		set({ currentMetadata: meta });
	},
	refetchLogs: async (backend: IBackendState) => {
		const { filter } = get();

		if (!filter) {
			return;
		}

		const runs = await backend.boardState.listRuns(
			filter.appId,
			filter.boardId,
			filter.nodeId,
			filter.from,
			filter.to,
			filter.status,
			filter.lastMeta,
			filter.offset,
			filter.limit,
		);

		set({ currentLogs: runs.toSorted((a, b) => b.start - a.start) });
	},
}));
