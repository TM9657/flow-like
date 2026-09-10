import type { BpmnContainer, BpmnProcess } from "../bpmn-model";
import { type Ctx, STEP_X, STEP_Y } from "./context";

/**
 * Where imported elements land. A file with diagram interchange keeps its own
 * arrangement, scaled up because flow-like nodes are wider than BPMN shapes; a
 * file without one is ranked along its sequence flows instead.
 */

export function computePositions(ctx: Ctx): void {
	const { shapes } = ctx.defs.diagram;
	let minX = Number.POSITIVE_INFINITY;
	let minY = Number.POSITIVE_INFINITY;
	for (const shape of shapes.values()) {
		if (shape.x < minX) minX = shape.x;
		if (shape.y < minY) minY = shape.y;
	}
	if (!Number.isFinite(minX)) minX = 0;
	if (!Number.isFinite(minY)) minY = 0;

	for (const [id, shape] of shapes) {
		ctx.positions.set(id, {
			x: Math.round((shape.x - minX) * ctx.scale.x),
			y: Math.round((shape.y - minY) * ctx.scale.y),
		});
	}

	let offsetY = 0;
	for (const process of ctx.defs.processes) {
		offsetY = autoPosition(ctx, process, 0, offsetY) + STEP_Y;
	}
}

/**
 * Ranks nodes that have no diagram shape along their sequence flows so a file
 * without BPMNDI still lands as a readable left-to-right graph.
 */
function autoPosition(
	ctx: Ctx,
	container: BpmnContainer,
	originX: number,
	originY: number,
): number {
	const nodes = container.flowNodes;
	const positioned = nodes.filter((n) => ctx.positions.has(n.id));
	if (positioned.length === nodes.length && nodes.length > 0) {
		let maxY = originY;
		for (const node of nodes) {
			const pos = ctx.positions.get(node.id);
			if (pos && pos.y > maxY) maxY = pos.y;
			if (node.body) autoPosition(ctx, node.body, 0, 0);
		}
		return maxY;
	}

	const rank = new Map<string, number>();
	const ids = new Set(nodes.map((n) => n.id));
	const outgoing = new Map<string, string[]>();
	const incomingCount = new Map<string, number>();
	for (const node of nodes) {
		outgoing.set(node.id, []);
		incomingCount.set(node.id, 0);
	}
	for (const flow of container.sequenceFlows) {
		if (!ids.has(flow.sourceRef) || !ids.has(flow.targetRef)) continue;
		outgoing.get(flow.sourceRef)?.push(flow.targetRef);
		incomingCount.set(
			flow.targetRef,
			(incomingCount.get(flow.targetRef) ?? 0) + 1,
		);
	}
	for (const node of nodes) {
		if (node.kind === "boundaryEvent" && node.attachedToRef) {
			incomingCount.set(node.id, 1);
		}
	}

	const queue: string[] = [];
	for (const node of nodes) {
		if ((incomingCount.get(node.id) ?? 0) === 0) {
			rank.set(node.id, 0);
			queue.push(node.id);
		}
	}
	const visits = new Map<string, number>();
	while (queue.length > 0) {
		const id = queue.shift() as string;
		const current = rank.get(id) ?? 0;
		for (const next of outgoing.get(id) ?? []) {
			const seen = (visits.get(next) ?? 0) + 1;
			visits.set(next, seen);
			if (seen > nodes.length) continue;
			const proposed = current + 1;
			if ((rank.get(next) ?? -1) < proposed) {
				rank.set(next, proposed);
				queue.push(next);
			}
		}
	}
	for (const node of nodes) {
		if (!rank.has(node.id)) {
			if (node.kind === "boundaryEvent" && node.attachedToRef) {
				rank.set(node.id, (rank.get(node.attachedToRef) ?? 0) + 1);
			} else {
				rank.set(node.id, 0);
			}
		}
	}

	const perRank = new Map<number, number>();
	let maxY = originY;
	for (const node of nodes) {
		if (ctx.positions.has(node.id)) continue;
		const r = rank.get(node.id) ?? 0;
		const row = perRank.get(r) ?? 0;
		perRank.set(r, row + 1);
		const pos = { x: originX + r * STEP_X, y: originY + row * STEP_Y };
		ctx.positions.set(node.id, pos);
		if (pos.y > maxY) maxY = pos.y;
	}
	for (const node of nodes) {
		if (node.body) {
			const own = ctx.positions.get(node.id) ?? { x: originX, y: originY };
			const bottom = autoPosition(ctx, node.body, own.x, own.y + STEP_Y);
			if (bottom > maxY) maxY = bottom;
		}
	}
	return maxY;
}

/** Where an element sits, falling back to the origin for one with no shape. */
export function nodePosition(ctx: Ctx, id: string): { x: number; y: number } {
	return ctx.positions.get(id) ?? { x: 0, y: 0 };
}

/** Where an element sits, or `undefined` when the file gave it no shape. */
export function positionOf(
	ctx: Ctx,
	id: string,
): { x: number; y: number } | undefined {
	return ctx.positions.get(id);
}

/** Pools are drawn for the participant, not the process it references. */
export function processPosition(
	ctx: Ctx,
	process: BpmnProcess,
): { x: number; y: number } {
	const participant = ctx.defs.collaboration?.participants.find(
		(p) => p.processRef === process.id,
	);
	return (
		positionOf(ctx, process.id) ??
		(participant ? positionOf(ctx, participant.id) : undefined) ?? {
			x: 0,
			y: 0,
		}
	);
}

export function sizeOf(
	ctx: Ctx,
	id: string,
): { width: number; height: number } {
	const shape = ctx.defs.diagram.shapes.get(id);
	if (!shape) return { width: 200, height: 80 };
	return {
		width: Math.max(120, Math.round(shape.width * ctx.scale.x)),
		height: Math.max(60, Math.round(shape.height * ctx.scale.y)),
	};
}
