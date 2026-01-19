import { create } from "zustand";

interface IRunExecutionState {
	runs: Map<
		string,
		{
			eventIds: string[];
			boardId: string;
			/** Currently executing nodes (unique) */
			nodes: Set<string>;
			/** Nodes that have finished executing (unique) */
			already_executed: Set<string>;
			/** Timestamp (ms since epoch) of the last node update event */
			lastNodeUpdateMs: number;
			/** Total number of node executions started (counts duplicates for loops) */
			totalExecutionsStarted: number;
			/** Total number of node executions completed (counts duplicates for loops) */
			totalExecutionsCompleted: number;
			/** The most recently active node ID */
			lastActiveNodeId: string | undefined;
		}
	>;
	pushUpdate(runId: string, events: IRunUpdateEvent[]): void;
	addRun: (runId: string, boardId: string, eventIds: string[]) => Promise<void>;
	removeRun: (runId: string) => void;
	addNodesOnRun: (runId: string, nodeIds: string[]) => void;
	removeNodesOnRun: (runId: string, nodeIds: string[]) => void;
	getTimeSinceLastUpdate: (runId: string) => number | null;
}

export interface IRunUpdateEvent {
	runId: string;
	nodeIds: string[];
	method: "remove" | "add" | "update";
}

export const useRunExecutionStore = create<IRunExecutionState>((set, get) => ({
	run_nodes: new Map(),
	runs: new Map(),
	pushUpdate: (runId: string, events: IRunUpdateEvent[]) => {
		const add_nodes = new Map();
		const remove_nodes = new Map();

		for (const payload of events) {
			if (payload.method === "add") {
				if (add_nodes.has(payload.runId)) {
					add_nodes.set(payload.runId, [
						...add_nodes.get(payload.runId),
						...payload.nodeIds,
					]);
					continue;
				}
				add_nodes.set(payload.runId, payload.nodeIds);
				continue;
			}

			if (remove_nodes.has(payload.runId)) {
				remove_nodes.set(payload.runId, [
					...remove_nodes.get(payload.runId),
					...payload.nodeIds,
				]);
				continue;
			}

			remove_nodes.set(payload.runId, payload.nodeIds);
		}

		// Update lastNodeUpdateMs for each run that received events
		const now = Date.now();
		set((state) => {
			const runs = new Map(state.runs);
			for (const run_id of [...add_nodes.keys(), ...remove_nodes.keys()]) {
				const run = runs.get(run_id);
				if (run) {
					runs.set(run_id, { ...run, lastNodeUpdateMs: now });
				}
			}
			return { runs };
		});

		add_nodes.forEach((node_ids, run_id) => {
			get().addNodesOnRun(run_id, node_ids);
		});

		remove_nodes.forEach((node_ids, run_id) => {
			get().removeNodesOnRun(run_id, node_ids);
		});
	},
	addRun: async (runId: string, boardId: string, eventIds: string[]) => {
		if (get().runs.has(runId)) {
			return;
		}

		set((state) => {
			const runs = new Map(state.runs);
			runs.set(runId, {
				eventIds,
				boardId,
				nodes: new Set(),
				already_executed: new Set(),
				lastNodeUpdateMs: Date.now(),
				totalExecutionsStarted: 0,
				totalExecutionsCompleted: 0,
				lastActiveNodeId: undefined,
			});
			return { runs };
		});
	},

	removeRun: (runId: string) =>
		set((state) => {
			const runs = new Map(state.runs);
			runs.delete(runId);
			return { runs };
		}),

	addNodesOnRun: (runId: string, nodeIds: string[]) =>
		set((state) => {
			const runs = new Map(state.runs);
			const run = runs.get(runId);
			if (!run) {
				return state;
			}

			const newNodes = new Set(run.nodes);
			nodeIds.forEach((nodeId) => newNodes.add(nodeId));

			// Track last active node (most recent in the batch)
			const lastActiveNodeId = nodeIds.length > 0 ? nodeIds[nodeIds.length - 1] : run.lastActiveNodeId;

			runs.set(runId, {
				...run,
				nodes: newNodes,
				lastNodeUpdateMs: Date.now(),
				// Count every execution start, not just unique nodes
				totalExecutionsStarted: run.totalExecutionsStarted + nodeIds.length,
				lastActiveNodeId,
			});
			return { runs };
		}),

	removeNodesOnRun: (runId: string, nodeIds: string[]) =>
		set((state) => {
			const runs = new Map(state.runs);
			const run = runs.get(runId);
			if (!run) {
				return state;
			}

			const newNodes = new Set(run.nodes);
			const newAlreadyExecuted = new Set(run.already_executed);
			nodeIds.forEach((nodeId) => newNodes.delete(nodeId));
			nodeIds.forEach((nodeId) => newAlreadyExecuted.add(nodeId));

			runs.set(runId, {
				...run,
				nodes: newNodes,
				already_executed: newAlreadyExecuted,
				lastNodeUpdateMs: Date.now(),
				// Count every execution completion, not just unique nodes
				totalExecutionsCompleted: run.totalExecutionsCompleted + nodeIds.length,
			});
			return { runs };
		}),

	getTimeSinceLastUpdate: (runId: string) => {
		const run = get().runs.get(runId);
		if (!run) return null;
		return Date.now() - run.lastNodeUpdateMs;
	},
}));
