import { createId } from "@paralleldrive/cuid2";
import type { IBoard, IComment, INode, IPin, IVariable } from "../../schema";
import { ICommentType, IPinType, IVariableType } from "../../schema";
import { type ILayer, ILayerType } from "../../schema/flow/board";
import {
	type CatalogIndex,
	connectPins,
	createVariable,
	now,
} from "../board-builder";
import type {
	BpmnDefinitions,
	BpmnFlowNode,
	BpmnSequenceFlow,
} from "../bpmn-model";
import {
	inputPin,
	outputPin,
	placeCatalogNode,
	setDefault,
} from "../bpmn-nodes";
import type { TranslationDiagnostic } from "../types";

/**
 * The state one BPMN translation carries, and the primitives every stage uses
 * to add to the fragment. Nothing here knows about a specific BPMN element,
 * which is what keeps the element modules free to import it.
 */

export const DEFAULT_SCALE = { x: 2.6, y: 2.2 };
/** Horizontal step between nodes the translator invents next to an element. */
export const STEP_X = 320;
export const STEP_Y = 160;
export const PLACEHOLDER_COLOR = "#FFA500";
export const COMPOSITION_COLOR = "#90CAF9";
export const SCOPE_COLOR = "#B39DDB";

export interface ExecPort {
	node: INode;
	pin: IPin;
}

/** Where sequence flows attach on the board for one BPMN flow node. */
export interface Anchor {
	/** Exec input every incoming flow attaches to, unless `entries` names one. */
	entry?: ExecPort;
	/** Per-incoming-flow exec inputs (joins that need one pin per flow). */
	entries?: Map<string, ExecPort>;
	/**
	 * Elements that are not nodes (merges, link events, embedded sub-process
	 * starts): incoming flows are redirected to the targets of these flows.
	 */
	through?: string[];
	/** The exec output a specific outgoing flow leaves from. */
	exits: Map<string, ExecPort>;
	/** Exec outputs for flows without a dedicated port. */
	defaultExits: ExecPort[];
}

/** What a translated activity offers to its wrappers (boundaries, loops, splits). */
export interface Implementation {
	entry: ExecPort;
	exits: ExecPort[];
	/** Exec output that fires when the implementation fails, if it has one. */
	errorExit?: ExecPort;
}

export interface Scope {
	/** Layer id nodes of this scope live in; `null` = fragment root. */
	layerId: string | null;
	/** Process the scope belongs to, which is what link events pair within. */
	processId: string;
	/** Enclosing sub-process activity, for boundary catch resolution. */
	activity?: BpmnFlowNode;
	parent?: Scope;
	/** Exec outputs of end events, which complete the enclosing activity. */
	endPorts: ExecPort[];
	/** Function layer whose boundary pins are the scope's signature. */
	functionLayer?: ILayer;
}

export interface Ctx {
	board: IBoard;
	catalog?: CatalogIndex;
	defs: BpmnDefinitions;
	diagnostics: TranslationDiagnostic[];
	scale: { x: number; y: number };
	stats: {
		totalNodes: number;
		directMapped: number;
		composed: number;
		todo: number;
		connections: number;
		variables: number;
	};
	nodesById: Map<string, BpmnFlowNode>;
	flowsById: Map<string, BpmnSequenceFlow>;
	anchors: Map<string, Anchor>;
	positions: Map<string, { x: number; y: number }>;
	/** Link catch events by link name, per process. */
	linkCatches: Map<string, BpmnFlowNode>;
	/** Boundary events keyed by the activity they hang on. */
	boundaries: Map<string, BpmnFlowNode[]>;
	variablesByData: Map<string, IVariable>;
	/** Processes that are called from a callActivity become functions. */
	functionByProcess: Map<string, ILayer>;
	usedFunctionNames: Set<string>;
	/** Function boundary inputs waiting for their first inner node. */
	pendingFunctionEntries: PendingFunctionEntry[];
}

export interface PendingFunctionEntry {
	pin: IPin;
	targetId: string;
	flowId: string;
}
export function connect(ctx: Ctx, from: ExecPort, to: ExecPort): void {
	connectPins(from.node, from.pin, to.node, to.pin);
	ctx.stats.connections += 1;
}

export function port(node: INode, pinName: string, type: IPinType): ExecPort {
	const pin =
		type === IPinType.Input
			? inputPin(node, pinName)
			: outputPin(node, pinName);
	if (!pin) {
		throw new Error(
			`BPMN import: node "${node.name}" has no ${type} pin "${pinName}"`,
		);
	}
	return { node, pin };
}

export function placeOrThrow(
	ctx: Ctx,
	name: string,
	opts: Parameters<typeof placeCatalogNode>[3],
): INode {
	const node = placeCatalogNode(ctx.board, ctx.catalog, name, opts);
	if (!node) {
		throw new Error(`BPMN import: catalog node "${name}" is unavailable`);
	}
	return node;
}

export function createLayer(
	ctx: Ctx,
	opts: {
		name: string;
		comment?: string;
		color: string;
		parentId: string | null;
		position: { x: number; y: number };
		type?: ILayerType;
	},
): ILayer {
	const layer: ILayer = {
		id: createId(),
		name: opts.name,
		type: opts.type ?? ILayerType.Collapsed,
		coordinates: [opts.position.x, opts.position.y, 0],
		comment: opts.comment ?? null,
		color: opts.color,
		parent_id: opts.parentId,
		nodes: {},
		variables: {},
		comments: {},
		pins: {},
		error: null,
		hash: null,
		category: null,
		in_coordinates: null,
		out_coordinates: null,
	};
	ctx.board.layers[layer.id] = layer;
	return layer;
}

export function addComment(
	ctx: Ctx,
	opts: {
		content: string;
		position: { x: number; y: number };
		layer: string | null;
		width?: number;
		height?: number;
		zIndex?: number;
		color?: string;
		nodeId?: string;
	},
): IComment {
	const comment: IComment = {
		id: createId(),
		content: opts.content,
		comment_type: ICommentType.Text,
		coordinates: [opts.position.x, opts.position.y, 0],
		timestamp: now(),
		author: "BPMN import",
		color: opts.color ?? null,
		width: opts.width ?? null,
		height: opts.height ?? null,
		z_index: opts.zIndex ?? null,
		is_locked: null,
		layer: opts.layer,
		hash: null,
		node_id: opts.nodeId ?? null,
	};
	ctx.board.comments[comment.id] = comment;
	return comment;
}

export function anchorOf(ctx: Ctx, id: string): Anchor {
	let anchor = ctx.anchors.get(id);
	if (!anchor) {
		anchor = { exits: new Map(), defaultExits: [] };
		ctx.anchors.set(id, anchor);
	}
	return anchor;
}

/**
 * Wires a data pin to a data pin by name. Both must exist — a composition that
 * cannot connect its own parts is a bug in the mapping, not in the file.
 */
export function connectData(
	from: INode,
	fromPin: string,
	to: INode,
	toPin: string,
): void {
	const source = outputPin(from, fromPin);
	const target = inputPin(to, toPin);
	if (!source || !target) {
		throw new Error(
			`BPMN import: cannot connect ${from.name}.${fromPin} to ${to.name}.${toPin}`,
		);
	}
	connectPins(from, source, to, target);
}

/**
 * The shape every element without a runtime equivalent takes: a log node
 * carrying the BPMN details, which runs and continues so the rest of the
 * process still executes. `terminal` marks an element nothing follows. The
 * caller counts it, because only the caller knows whether it stands in for the
 * element or maps it.
 */
export function placeLog(
	ctx: Ctx,
	level: "log_info" | "log_warning" | "log_error",
	scope: Scope,
	position: { x: number; y: number },
	opts: {
		comment: string;
		message: string;
		terminal?: boolean;
		layer?: string;
	},
): Implementation {
	const node = placeOrThrow(ctx, level, {
		x: position.x,
		y: position.y,
		layer: opts.layer ?? scope.layerId,
		comment: opts.comment,
	});
	setDefault(node, "message", opts.message);
	return {
		entry: port(node, "exec_in", IPinType.Input),
		exits: opts.terminal ? [] : [port(node, "exec_out", IPinType.Output)],
	};
}

/** Adds a board variable for a BPMN data object, reusing one already made for it. */
export function ensureVariable(
	ctx: Ctx,
	dataId: string,
	name: string,
	description?: string,
): IVariable {
	const existing = ctx.variablesByData.get(dataId);
	if (existing) return existing;
	const taken = new Set(
		Object.values(ctx.board.variables).map((v) => v.name.toLowerCase()),
	);
	let unique = name;
	for (let i = 2; taken.has(unique.toLowerCase()); i += 1) {
		unique = `${name} ${i}`;
	}
	const variable = createVariable({
		name: unique,
		description: description ?? `BPMN data object ${dataId}`,
		dataType: IVariableType.Generic,
	});
	ctx.board.variables[variable.id] = variable;
	ctx.variablesByData.set(dataId, variable);
	ctx.stats.variables += 1;
	return variable;
}
