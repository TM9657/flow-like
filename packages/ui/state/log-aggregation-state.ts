import { create } from "zustand";
import type { IBackendState } from "./backend-state";
import type { ILogLevel, ILogMetadata } from "../lib";

interface ILogAggregationFilter {
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
    setFilter(backend: IBackendState, filter: ILogAggregationFilter) : Promise<void>;
    setCurrentMetadata: (meta?: ILogMetadata) => void;
}

export const useLogAggregation = create<ILogAggregationState>((set, get) => ({
    currentLogs: [],
    filter: undefined,
    currentMetadata: undefined,
    setFilter: async (backend: IBackendState, filter: ILogAggregationFilter) => {
        set({ filter });
        const runs = await backend.listRuns(
            filter.appId,
            filter.boardId,
            filter.nodeId,
            filter.from,
            filter.to,
            filter.status,
            filter.limit,
            filter.offset,
            filter.lastMeta,
        )

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

        const runs = await backend.listRuns(
            filter.appId,
            filter.boardId,
            filter.nodeId,
            filter.from,
            filter.to,
            filter.status,
            filter.limit,
            filter.offset,
            filter.lastMeta,
        );

        set({ currentLogs: runs.toSorted((a, b) => b.start - a.start) });
    }
}));