export interface NodeRunState {
	nodes: ReadonlySet<string>;
	already_executed: ReadonlySet<string>;
}

/** A live invocation owns the activity timer and Stop action before an older completed invocation. */
export function nodeExecutionState(
	runs: ReadonlyMap<string, NodeRunState>,
	nodeId: string,
): { status: "running" | "done" | "none"; runId?: string } {
	for (const [runId, run] of runs) {
		if (run.nodes.has(nodeId)) return { status: "running", runId };
	}
	for (const [runId, run] of runs) {
		if (run.already_executed.has(nodeId)) return { status: "done", runId };
	}
	return { status: "none" };
}
