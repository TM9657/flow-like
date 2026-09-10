import { IPinType, IVariableType } from "../../schema";
import { info, warn } from "../board-builder";
import type { BpmnFlowNode, BpmnSequenceFlow } from "../bpmn-model";
import {
	ensurePins,
	mintPin,
	setDefault,
	switchCasePinNames,
} from "../bpmn-nodes";
import {
	type Anchor,
	type Ctx,
	type ExecPort,
	STEP_X,
	STEP_Y,
	type Scope,
	anchorOf,
	connect,
	placeOrThrow,
	port,
} from "./context";
import { label } from "./describe";
import { nodePosition } from "./layout";

/**
 * Gateways and every other place one element continues into several. BPMN
 * decides which branches run; flow-like decides it with nodes, so each
 * semantics gets the node that matches it.
 */

export type SplitSemantics = "exclusive" | "inclusive" | "parallel";

export function translateGateway(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
): void {
	const position = nodePosition(ctx, node.id);
	const anchor = anchorOf(ctx, node.id);
	ctx.stats.totalNodes += 1;
	const incoming = node.incoming.length;
	const outgoing = node.outgoing.length;
	const name = label(node);

	let semantics: SplitSemantics;
	switch (node.kind) {
		case "parallelGateway":
		case "eventBasedGateway":
			semantics = "parallel";
			break;
		case "inclusiveGateway":
		case "complexGateway":
			semantics = "inclusive";
			break;
		default:
			semantics = "exclusive";
	}

	if (node.kind === "complexGateway") {
		warn(
			ctx.diagnostics,
			`Complex gateway "${name}" is treated as an inclusive gateway`,
			node.id,
			node.name,
		);
	}
	if (node.kind === "eventBasedGateway") {
		warn(
			ctx.diagnostics,
			`Event-based gateway "${name}" starts every branch; the first event does not cancel the others`,
			node.id,
			node.name,
		);
	}
	if (outgoing === 0) {
		warn(
			ctx.diagnostics,
			`Gateway "${name}" has no outgoing flow`,
			node.id,
			node.name,
		);
	}

	// Joins: a parallel join needs a Gather with one input per incoming flow.
	// Exclusive and inclusive joins need no node — an exec input accepts many
	// sources — though an inclusive join then continues once per branch.
	let joinExit: ExecPort | undefined;
	if (node.kind === "parallelGateway" && incoming > 1) {
		const gather = placeOrThrow(ctx, "control_gather", {
			x: position.x,
			y: position.y,
			layer: scope.layerId,
			comment: `${name} (join ${incoming})`,
		});
		const ins = ensurePins(
			gather,
			"exec_in",
			IPinType.Input,
			Math.max(2, incoming),
		);
		anchor.entries = new Map(
			node.incoming.map((flowId, i) => [flowId, { node: gather, pin: ins[i] }]),
		);
		joinExit = port(gather, "exec_done", IPinType.Output);
	} else if (
		incoming > 1 &&
		node.kind === "inclusiveGateway" &&
		outgoing <= 1
	) {
		warn(
			ctx.diagnostics,
			`Inclusive join "${name}" continues once per arriving branch; use Gather if the branches run in parallel`,
			node.id,
			node.name,
		);
	}

	ctx.stats.directMapped += 1;

	if (outgoing <= 1) {
		if (joinExit) anchor.defaultExits = [joinExit];
		else anchor.through = node.outgoing;
		return;
	}

	const entry = buildSplit(
		ctx,
		node,
		anchor,
		scope,
		semantics,
		joinExit,
		position,
	);
	if (!joinExit && entry) anchor.entry = entry;
}

/** After an element with several outgoing flows, fan out according to BPMN semantics. */
/**
 * After an element with several outgoing flows, fan out the way BPMN says the
 * element does: a gateway states it, anything else is an implicit AND-split.
 */
export function splitOutgoing(
	ctx: Ctx,
	node: BpmnFlowNode,
	anchor: Anchor,
	scope: Scope,
	semantics: SplitSemantics,
): void {
	if (node.outgoing.length <= 1 || anchor.through) return;
	const position = nodePosition(ctx, node.id);
	const from = anchor.defaultExits;
	if (from.length === 0) return;
	anchor.defaultExits = [];
	buildSplit(ctx, node, anchor, scope, semantics, undefined, position, from);
}

/**
 * Materialises a split. Returns the exec input the incoming flows should use
 * when no `from` ports are given (gateway case); otherwise wires from them.
 */
function buildSplit(
	ctx: Ctx,
	node: BpmnFlowNode,
	anchor: Anchor,
	scope: Scope,
	semantics: SplitSemantics,
	joinExit: ExecPort | undefined,
	position: { x: number; y: number },
	from?: ExecPort[],
): ExecPort | undefined {
	const flows = node.outgoing
		.map((id) => ctx.flowsById.get(id))
		.filter((f): f is BpmnSequenceFlow => Boolean(f));
	const sources = from ?? (joinExit ? [joinExit] : []);
	const conditioned = flows.filter(
		(f) => f.condition && f.id !== node.defaultFlow,
	);
	const unconditioned = flows.filter(
		(f) => !f.condition && f.id !== node.defaultFlow,
	);
	const defaultFlow = flows.find((f) => f.id === node.defaultFlow);
	const name = label(node);
	const x = position.x + STEP_X / 2;
	const y = position.y;

	const attach = (entry: ExecPort) => {
		for (const source of sources) connect(ctx, source, entry);
	};

	// A gateway whose branches are labelled but carry no condition is the usual
	// shape of a diagram drawn to be read: the labels are the choice. One Switch
	// over those labels says that, where a chain of Branch nodes would ask for a
	// boolean per branch that the file never had.
	const labelled = flows.filter((f) => f.name?.trim() && f !== defaultFlow);
	if (
		semantics === "exclusive" &&
		conditioned.length === 0 &&
		labelled.length > 1
	) {
		return buildLabelSwitch(ctx, node, anchor, scope, {
			flows: labelled,
			defaultFlow,
			position: { x, y },
			attach,
		});
	}

	if (semantics === "exclusive") {
		const ordered = [
			...conditioned,
			...unconditioned.filter((f) => f !== defaultFlow),
		];
		if (ordered.length === 0 && defaultFlow) ordered.push(defaultFlow);
		if (conditioned.length === 0 && ordered.length > 1) {
			warn(
				ctx.diagnostics,
				`Exclusive gateway "${name}" states no condition on any of its ${ordered.length} branches; they are tested in the order they were drawn`,
				node.id,
				node.name,
			);
		}
		let entry: ExecPort | undefined;
		let previousFalse: ExecPort | undefined;
		ordered.forEach((flow, i) => {
			const branch = placeOrThrow(ctx, "control_branch", {
				x: x + i * STEP_X,
				y: y + i * (STEP_Y / 2),
				layer: scope.layerId,
				comment: `${name}${flow.name ? ` → ${flow.name}` : ""}\ncondition: ${flow.condition ?? "(none)"}\nWire the condition`,
			});
			setDefault(branch, "condition", !flow.condition);
			const branchIn = port(branch, "exec_in", IPinType.Input);
			if (previousFalse) connect(ctx, previousFalse, branchIn);
			else entry = branchIn;
			anchor.exits.set(flow.id, port(branch, "true", IPinType.Output));
			previousFalse = port(branch, "false", IPinType.Output);
		});
		if (defaultFlow && previousFalse && !ordered.includes(defaultFlow)) {
			anchor.exits.set(defaultFlow.id, previousFalse);
		} else if (previousFalse && !defaultFlow) {
			warn(
				ctx.diagnostics,
				`Exclusive gateway "${name}" has no default flow; nothing runs when no condition matches`,
				node.id,
				node.name,
			);
		}
		for (const flow of conditioned) {
			info(
				ctx.diagnostics,
				`Condition on "${flow.name ?? flow.id}": ${flow.condition}`,
				node.id,
				node.name,
			);
		}
		if (entry) attach(entry);
		return entry;
	}

	// Parallel and inclusive: one Parallel Execution output per flow; inclusive
	// conditions gate their branch through a Branch node.
	const fork = placeOrThrow(ctx, "control_par_execution", {
		x,
		y,
		layer: scope.layerId,
		comment: `${name}${semantics === "inclusive" ? " (inclusive)" : ""}`,
	});
	const outs = ensurePins(
		fork,
		"exec_out",
		IPinType.Output,
		Math.max(2, flows.length),
	);
	flows.forEach((flow, i) => {
		const out: ExecPort = { node: fork, pin: outs[i] };
		if (semantics === "inclusive" && flow.condition && flow !== defaultFlow) {
			const branch = placeOrThrow(ctx, "control_branch", {
				x: x + STEP_X,
				y: y + i * (STEP_Y / 2),
				layer: scope.layerId,
				comment: `${name}${flow.name ? ` → ${flow.name}` : ""}\ncondition: ${flow.condition}\nWire the condition`,
			});
			setDefault(branch, "condition", false);
			connect(ctx, out, port(branch, "exec_in", IPinType.Input));
			anchor.exits.set(flow.id, port(branch, "true", IPinType.Output));
			info(
				ctx.diagnostics,
				`Condition on "${flow.name ?? flow.id}": ${flow.condition}`,
				node.id,
				node.name,
			);
		} else {
			anchor.exits.set(flow.id, out);
			if (semantics === "inclusive" && flow === defaultFlow) {
				warn(
					ctx.diagnostics,
					`Default flow of inclusive gateway "${name}" always runs; guard it manually`,
					node.id,
					node.name,
				);
			}
		}
	});
	const entry = port(fork, "exec_in", IPinType.Input);
	attach(entry);
	return entry;
}

/**
 * An exclusive gateway whose branches are named rather than conditioned, as one
 * `control_switch`: the branch labels become its cases, so the only thing left
 * to wire is the value they are compared against.
 */
function buildLabelSwitch(
	ctx: Ctx,
	node: BpmnFlowNode,
	anchor: Anchor,
	scope: Scope,
	opts: {
		flows: BpmnSequenceFlow[];
		defaultFlow: BpmnSequenceFlow | undefined;
		position: { x: number; y: number };
		attach: (entry: ExecPort) => void;
	},
): ExecPort {
	const cases = opts.flows.map((flow) => (flow.name as string).trim());
	const switchNode = placeOrThrow(ctx, "control_switch", {
		x: opts.position.x,
		y: opts.position.y,
		layer: scope.layerId,
		comment: `${label(node)}\ncases: ${cases.join(", ")}\nWire the value the cases are compared against`,
	});
	setDefault(switchNode, "cases", cases.join(","));

	// Minted here so the branches can be wired now; `on_update` adopts pins that
	// already carry the right name and only refreshes their label.
	switchCasePinNames(cases).forEach((pinName, index) => {
		const pin = mintPin(switchNode, {
			name: pinName,
			friendly: cases[index],
			type: IPinType.Output,
			data: IVariableType.Execution,
		});
		anchor.exits.set(opts.flows[index].id, { node: switchNode, pin });
	});

	const fallback = port(switchNode, "default", IPinType.Output);
	if (opts.defaultFlow) anchor.exits.set(opts.defaultFlow.id, fallback);

	info(
		ctx.diagnostics,
		`Exclusive gateway "${label(node)}" branches on ${cases.map((c) => `"${c}"`).join(", ")}`,
		node.id,
		node.name,
	);

	const entry = port(switchNode, "exec_in", IPinType.Input);
	opts.attach(entry);
	return entry;
}
