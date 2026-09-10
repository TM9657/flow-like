import { info, warn } from "../board-builder";
import type { BpmnSequenceFlow } from "../bpmn-model";
import { type Ctx, type ExecPort, connect } from "./context";
import { label } from "./describe";

/**
 * Turning sequence flows into wires, once every element has an anchor. Doing
 * it in one pass at the end is what lets an element be translated before the
 * elements it points at exist.
 */

/** Follows pass-through anchors to the exec input a flow into `targetId` should land on. */
export function resolveEntry(
	ctx: Ctx,
	targetId: string,
	flowId: string | undefined,
	visited: Set<string> = new Set(),
): ExecPort | undefined {
	if (visited.has(targetId)) return undefined;
	visited.add(targetId);
	const anchor = ctx.anchors.get(targetId);
	if (!anchor) return undefined;
	if (flowId && anchor.entries?.has(flowId)) return anchor.entries.get(flowId);
	if (anchor.entry) return anchor.entry;
	if (anchor.through && anchor.through.length > 0) {
		const nextFlow = ctx.flowsById.get(anchor.through[0]);
		if (!nextFlow) return undefined;
		return resolveEntry(ctx, nextFlow.targetRef, nextFlow.id, visited);
	}
	return undefined;
}

export function wireSequenceFlows(ctx: Ctx): void {
	for (const entry of ctx.pendingFunctionEntries) {
		const target = resolveEntry(ctx, entry.targetId, entry.flowId);
		if (!target) continue;
		entry.pin.connected_to.push(target.pin.id);
		target.pin.depends_on.push(entry.pin.id);
		ctx.stats.connections += 1;
	}

	for (const flow of ctx.flowsById.values()) {
		const source = ctx.anchors.get(flow.sourceRef);
		if (!source || source.through) continue;
		const dedicated = source.exits.get(flow.id);
		const ports = dedicated ? [dedicated] : source.defaultExits;
		if (ports.length === 0) continue;

		const sourceNode = ctx.nodesById.get(flow.sourceRef);
		const targetNode = ctx.nodesById.get(flow.targetRef);
		const target = resolveEntry(ctx, flow.targetRef, flow.id);
		if (!target) {
			// An event sub-process is entered by its own start event, so a flow into
			// one is the file's mistake rather than ours.
			if (
				targetNode &&
				!(targetNode.kind === "subProcess" && targetNode.triggeredByEvent)
			) {
				warn(
					ctx.diagnostics,
					`Sequence flow ${flow.id} into "${label(targetNode)}" could not be connected`,
					flow.sourceRef,
				);
			}
			continue;
		}

		for (const from of ports) {
			// An execution output drives exactly one target; a second flow out of
			// the same port means the split was not materialised.
			if (
				from.pin.connected_to.length > 0 &&
				!from.pin.connected_to.includes(target.pin.id)
			) {
				warn(
					ctx.diagnostics,
					`Flow ${flow.id} from "${sourceNode ? label(sourceNode) : flow.sourceRef}" needs a fork; only one target per execution output`,
					flow.sourceRef,
				);
				continue;
			}
			connect(ctx, from, target);
		}

		if (targetNode && isBackEdge(ctx, flow)) {
			info(
				ctx.diagnostics,
				`Sequence flow ${flow.id} loops back to "${label(targetNode)}"; FlowScript text cannot show cycles`,
				flow.sourceRef,
			);
		}
	}
}

/** Whether following `flow` can lead back to where it started. */
function isBackEdge(ctx: Ctx, flow: BpmnSequenceFlow): boolean {
	const seen = new Set<string>();
	const stack = [flow.targetRef];
	while (stack.length > 0) {
		const id = stack.pop() as string;
		if (id === flow.sourceRef) return true;
		if (seen.has(id)) continue;
		seen.add(id);
		const node = ctx.nodesById.get(id);
		for (const out of node?.outgoing ?? []) {
			const next = ctx.flowsById.get(out);
			if (next && next.id !== flow.id) stack.push(next.targetRef);
		}
	}
	return false;
}
