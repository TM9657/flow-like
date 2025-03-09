import { type Event, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import type { ITrace } from "../lib";

interface IRunExecutionState {
	runs: Map<
		string,
		{
			unlistenFn: UnlistenFn;
			eventIds: string[];
			boardId: string;
			nodes: Set<string>;
			already_executed: Set<string>;
		}
	>;
	addRun: (runId: string, boardId: string, eventIds: string[]) => Promise<void>;
	removeRun: (runId: string) => void;
	addNodesOnRun: (runId: string, nodeIds: string[]) => void;
	removeNodesOnRun: (runId: string, nodeIds: string[]) => void;
}

export interface IRunUpdateEvent {
	run_id: string;
	node_ids: string[];
	method: "remove" | "add" | "update";
	logs: ITrace[];
}

export const useRunExecutionStore = create<IRunExecutionState>((set, get) => ({
	run_nodes: new Map(),
	runs: new Map(),
	addRun: async (runId: string, boardId: string, eventIds: string[]) => {
		if (get().runs.has(runId)) {
			return;
		}

		const unlisten = await listen(
			`run:${runId}`,
			(event: Event<IRunUpdateEvent[]>) => {
				if (event.payload.length > 1) console.log(event.payload.length);

				const add_nodes = new Map();
				const remove_nodes = new Map();

				for (const payload of event.payload) {
					if (payload.method === "add") {
						if (add_nodes.has(payload.run_id)) {
							add_nodes.set(payload.run_id, [
								...add_nodes.get(payload.run_id),
								...payload.node_ids,
							]);
							continue;
						}
						add_nodes.set(payload.run_id, payload.node_ids);
						continue;
					}

					if (remove_nodes.has(payload.run_id)) {
						remove_nodes.set(payload.run_id, [
							...remove_nodes.get(payload.run_id),
							...payload.node_ids,
						]);
						continue;
					}

					remove_nodes.set(payload.run_id, payload.node_ids);
				}

				add_nodes.forEach((node_ids, run_id) => {
					get().addNodesOnRun(run_id, node_ids);
				});

				remove_nodes.forEach((node_ids, run_id) => {
					get().removeNodesOnRun(run_id, node_ids);
				});
			},
		);

		set((state) => {
			const runs = new Map(state.runs);
			runs.set(runId, {
				unlistenFn: unlisten,
				eventIds,
				boardId,
				nodes: new Set(),
				already_executed: new Set(),
			});
			return { runs };
		});
	},

	removeRun: (runId: string) =>
		set((state) => {
			const runs = new Map(state.runs);
			runs.get(runId)?.unlistenFn();
			runs.delete(runId);
			return { runs };
		}),

	addNodesOnRun: (runId: string, nodeIds: string[]) =>
		set((state) => {
			let runs = new Map(state.runs);
			const run = runs.get(runId);
			if (!run) {
				return state;
			}

			nodeIds.forEach((nodeId) => run.nodes.add(nodeId));
			runs.set(runId, run);
			return { runs };
		}),

	removeNodesOnRun: (runId: string, nodeIds: string[]) =>
		set((state) => {
			const runs = new Map(state.runs);
			const run = runs.get(runId);
			if (!run) {
				return state;
			}

			nodeIds.forEach((nodeId) => run.nodes.delete(nodeId));
			nodeIds.forEach((nodeId) => run.already_executed.add(nodeId));

			runs.set(runId, run);
			return { runs };
		}),
}));
