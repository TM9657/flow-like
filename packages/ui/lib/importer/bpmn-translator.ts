import type { INode } from "../schema";
import { IPinType, IVariableType } from "../schema";
import { type ILayer, ILayerType } from "../schema/flow/board";
import {
	buildCatalogIndex,
	createEmptyBoard,
	diagError,
	info,
	warn,
} from "./board-builder";
import {
	type BpmnAssociation,
	type BpmnContainer,
	type BpmnDefinitions,
	type BpmnEventDefinition,
	type BpmnFlowNode,
	type BpmnGroup,
	type BpmnLane,
	type BpmnProcess,
	type BpmnTextAnnotation,
	findExtension,
	isEventKind,
	isGatewayKind,
	iso8601CycleDurationToSeconds,
	iso8601DurationToSeconds,
	labelDurationToSeconds,
	walkFlowNodes,
	walkSequenceFlows,
} from "./bpmn-model";
import {
	catalogHas,
	ensurePins,
	execPinSpec,
	mintPin,
	setDefault,
} from "./bpmn-nodes";
import {
	COMPOSITION_COLOR,
	type Ctx,
	DEFAULT_SCALE,
	type ExecPort,
	type Implementation,
	PLACEHOLDER_COLOR,
	SCOPE_COLOR,
	STEP_X,
	STEP_Y,
	type Scope,
	addComment,
	anchorOf,
	connect,
	connectData,
	createLayer,
	ensureVariable,
	placeLog,
	placeOrThrow,
	port,
} from "./bpmn/context";
import {
	describeEventDef,
	describeNode,
	describeProcess,
	humanKind,
	label,
} from "./bpmn/describe";
import { splitOutgoing, translateGateway } from "./bpmn/gateways";
import {
	computePositions,
	nodePosition,
	positionOf,
	processPosition,
	sizeOf,
} from "./bpmn/layout";
import { resolveEntry, wireSequenceFlows } from "./bpmn/wiring";
import type { TranslationDiagnostic, TranslationResult } from "./types";

/**
 * BPMN 2.0 → flow-like board fragment.
 *
 * Every BPMN element becomes *something* on the board: a catalog node when the
 * semantics map cleanly (branches, forks, joins, timers, loops, logs), a
 * composition of catalog nodes when a vendor extension tells us what a task
 * does (HTTP, mail, LLM), and otherwise a placeholder layer named after the
 * element with the BPMN details in its comment, so the process shape is
 * preserved and each placeholder can be opened and modelled in place.
 *
 * The result is a fragment (top-level items have `layer: null`) meant for one
 * `copyPasteCommand`, plus the variables it references.
 */
export function translateBpmn(
	defs: BpmnDefinitions,
	catalog?: INode[],
): TranslationResult {
	const diagnostics: TranslationDiagnostic[] = [];
	const title =
		defs.name ??
		defs.collaboration?.name ??
		(defs.processes.length === 1
			? (defs.processes[0].name ?? defs.processes[0].id)
			: undefined) ??
		"Imported BPMN process";
	const board = createEmptyBoard(
		title,
		`Imported from BPMN${defs.exporter ? ` (${defs.exporter})` : ""}`,
	);
	const ctx: Ctx = {
		board,
		catalog: catalog ? buildCatalogIndex(catalog) : undefined,
		defs,
		diagnostics,
		scale: DEFAULT_SCALE,
		stats: {
			totalNodes: 0,
			directMapped: 0,
			composed: 0,
			todo: 0,
			connections: 0,
			variables: 0,
		},
		nodesById: new Map(),
		flowsById: new Map(),
		anchors: new Map(),
		positions: new Map(),
		linkCatches: new Map(),
		boundaries: new Map(),
		variablesByData: new Map(),
		functionByProcess: new Map(),
		usedFunctionNames: new Set(),
		pendingFunctionEntries: [],
	};

	info(diagnostics, `Translating BPMN definitions "${title}"`);
	if (ctx.catalog) {
		info(
			diagnostics,
			`Using catalog with ${ctx.catalog.size} node definitions`,
		);
	}

	for (const process of defs.processes) {
		for (const node of walkFlowNodes(process)) {
			// Ids are unique in valid BPMN; a repeat means one of the two elements
			// loses its edges, so say which rather than importing a silent mess.
			if (ctx.nodesById.has(node.id)) {
				diagError(
					diagnostics,
					`Duplicate element id "${node.id}"; the later ${humanKind(node.kind)} may not be connected correctly`,
					node.id,
					node.name,
				);
			}
			ctx.nodesById.set(node.id, node);
			if (node.kind === "boundaryEvent" && node.attachedToRef) {
				const list = ctx.boundaries.get(node.attachedToRef) ?? [];
				list.push(node);
				ctx.boundaries.set(node.attachedToRef, list);
			}
			const link = node.eventDefinitions.find((d) => d.kind === "link");
			if (node.kind === "intermediateCatchEvent" && link?.linkName) {
				ctx.linkCatches.set(`${process.id}::${link.linkName}`, node);
			}
		}
		for (const flow of walkSequenceFlows(process)) {
			ctx.flowsById.set(flow.id, flow);
		}
	}

	computePositions(ctx);

	if (defs.processes.length === 0) {
		diagError(diagnostics, "The BPMN file contains no process");
	}

	const called = new Set<string>();
	for (const node of ctx.nodesById.values()) {
		if (node.kind === "callActivity" && node.calledElement) {
			called.add(node.calledElement);
		}
	}

	// Every function layer exists before any body is translated, so a called
	// process that itself calls another still finds a layer to point at.
	const functions = defs.processes
		.filter((process) => called.has(process.id))
		.map((process) => ({ process, layer: createFunctionLayer(ctx, process) }));
	for (const { process, layer } of functions) {
		fillFunctionLayer(ctx, process, layer);
	}

	// Several root processes are several pools, and a pool is a scope of its own;
	// a single process is the board itself and needs no wrapper.
	const rootProcesses = defs.processes.filter((p) => !called.has(p.id));
	const useProcessLayers = rootProcesses.length > 1;

	for (const process of rootProcesses) {
		let scope: Scope = {
			layerId: null,
			processId: process.id,
			endPorts: [],
		};
		if (useProcessLayers) {
			const layer = createLayer(ctx, {
				name: process.participantName ?? process.name ?? process.id,
				comment: describeProcess(process),
				color: SCOPE_COLOR,
				parentId: null,
				position: processPosition(ctx, process),
			});
			scope = { layerId: layer.id, processId: process.id, endPorts: [] };
		}
		translateContainer(ctx, process, scope);
		translateLanes(ctx, process.lanes, scope);
		ensureProcessEntry(ctx, process, scope);
	}

	wireSequenceFlows(ctx);
	reportUnreachable(ctx);
	translateCollaborationArtifacts(ctx);

	const status: TranslationResult["status"] = diagnostics.some(
		(d) => d.level === "error",
	)
		? ctx.stats.totalNodes > 0
			? "partial"
			: "error"
		: ctx.stats.todo > 0
			? "partial"
			: "success";

	info(
		diagnostics,
		`Translated ${ctx.stats.totalNodes} elements: ${ctx.stats.directMapped} mapped, ${ctx.stats.composed} composed, ${ctx.stats.todo} placeholders, ${ctx.stats.connections} connections`,
	);

	return { format: "bpmn", status, board, diagnostics, stats: ctx.stats };
}

// ───────────────────────────── containers ─────────────────────────────

function translateContainer(
	ctx: Ctx,
	container: BpmnContainer,
	scope: Scope,
): void {
	for (const data of container.dataObjects) {
		// A reference and the object it points at are one variable; the name is on
		// whichever of the two carries it.
		const key = data.ref ?? data.id;
		const named =
			data.name ??
			container.dataObjects.find(
				(d) => (d.id === key || d.ref === key) && d.name,
			)?.name ??
			key;
		ctx.variablesByData.set(
			data.id,
			ensureVariable(ctx, key, named, data.documentation),
		);
	}

	// Boundary events last: they are attached while their host is translated.
	const ordered = [
		...container.flowNodes.filter((n) => n.kind !== "boundaryEvent"),
		...container.flowNodes.filter((n) => n.kind === "boundaryEvent"),
	];
	for (const node of ordered) {
		try {
			translateFlowNode(ctx, node, scope);
		} catch (error) {
			diagError(
				ctx.diagnostics,
				`Failed to translate ${humanKind(node.kind)} "${label(node)}": ${error instanceof Error ? error.message : String(error)}`,
				node.id,
				node.name,
			);
		}
	}

	addArtifactComments(ctx, container, scope.layerId, { x: 0, y: -STEP_Y });
}

/**
 * Text annotations and groups as board comments. An annotation without its own
 * diagram shape is placed above whatever it is associated with.
 */
function addArtifactComments(
	ctx: Ctx,
	artifacts: {
		annotations: BpmnTextAnnotation[];
		associations: BpmnAssociation[];
		groups: BpmnGroup[];
	},
	layerId: string | null,
	fallback: { x: number; y: number },
): void {
	for (const annotation of artifacts.annotations) {
		const association = artifacts.associations.find(
			(a) => a.sourceRef === annotation.id || a.targetRef === annotation.id,
		);
		const attachedTo =
			association?.sourceRef === annotation.id
				? association?.targetRef
				: association?.sourceRef;
		const own = positionOf(ctx, annotation.id);
		const near = attachedTo ? positionOf(ctx, attachedTo) : undefined;
		const size = own ? sizeOf(ctx, annotation.id) : undefined;
		addComment(ctx, {
			content: annotation.text ?? "",
			position: own ?? (near ? { x: near.x, y: near.y - STEP_Y } : fallback),
			layer: layerId,
			width: size?.width,
			height: size?.height,
		});
	}
	for (const group of artifacts.groups) {
		const own = positionOf(ctx, group.id);
		if (!own) continue;
		const size = sizeOf(ctx, group.id);
		addComment(ctx, {
			content: group.name ?? "Group",
			position: own,
			layer: layerId,
			width: size.width,
			height: size.height,
			zIndex: -1,
		});
	}
}

/**
 * Lanes are roles, not containers: nodes wire freely across them, so a lane
 * becomes a labelled box behind the nodes rather than a layer.
 */
function translateLanes(ctx: Ctx, lanes: BpmnLane[], scope: Scope): void {
	for (const lane of lanes) {
		const own = positionOf(ctx, lane.id);
		if (own) {
			const size = sizeOf(ctx, lane.id);
			addComment(ctx, {
				content: lane.name ?? "Lane",
				position: own,
				layer: scope.layerId,
				width: size.width,
				height: size.height,
				zIndex: -2,
			});
		} else if (lane.name) {
			info(
				ctx.diagnostics,
				`Lane "${lane.name}" holds: ${lane.flowNodeRefs.join(", ") || "nothing"}`,
			);
		}
		translateLanes(ctx, lane.children, scope);
	}
}

/** A process called by a callActivity becomes a Function layer with an exec in/out signature. */
function createFunctionLayer(ctx: Ctx, process: BpmnProcess): ILayer {
	const baseName = process.name ?? process.id;
	let name = baseName;
	for (let i = 2; ctx.usedFunctionNames.has(name.toLowerCase()); i += 1) {
		name = `${baseName} ${i}`;
	}
	ctx.usedFunctionNames.add(name.toLowerCase());

	const layer = createLayer(ctx, {
		name,
		comment: describeProcess(process),
		color: SCOPE_COLOR,
		parentId: null,
		position: processPosition(ctx, process),
		type: ILayerType.Function,
	});
	// The boundary pins are the function's signature: `control_call_function`
	// mirrors them onto the call node by name.
	mintPin(layer, execPinSpec("exec_in", "Input", IPinType.Input));
	mintPin(layer, execPinSpec("exec_out", "Output", IPinType.Output));
	ctx.functionByProcess.set(process.id, layer);
	return layer;
}

function fillFunctionLayer(
	ctx: Ctx,
	process: BpmnProcess,
	layer: ILayer,
): void {
	const scope: Scope = {
		layerId: layer.id,
		processId: process.id,
		endPorts: [],
		functionLayer: layer,
	};
	translateContainer(ctx, process, scope);
	translateLanes(ctx, process.lanes, scope);

	if (process.isExecutable) {
		info(
			ctx.diagnostics,
			`Process "${process.name ?? process.id}" is called from a call activity and was imported as a function; its own start events do not trigger runs`,
		);
	}
}

/**
 * Elements nothing leads to. In a collaboration these are usually the ones the
 * file starts with a message flow from another pool, which is not execution and
 * so cannot be wired — naming them beats leaving a node that never runs.
 */
function reportUnreachable(ctx: Ctx): void {
	const messageFlows = ctx.defs.collaboration?.messageFlows ?? [];
	for (const [elementId, anchor] of ctx.anchors) {
		const entry = anchor.entry;
		if (!entry || entry.pin.depends_on.length > 0) continue;
		if (entry.node.start) continue;
		const node = ctx.nodesById.get(elementId);
		if (!node || node.kind === "boundaryEvent") continue;

		const incoming = messageFlows.find((flow) => flow.targetRef === elementId);
		const sender = incoming
			? (ctx.defs.collaboration?.participants.find(
					(p) => p.id === incoming.sourceRef,
				)?.name ??
				(ctx.nodesById.get(incoming.sourceRef)
					? label(ctx.nodesById.get(incoming.sourceRef) as BpmnFlowNode)
					: incoming.sourceRef))
			: undefined;
		warn(
			ctx.diagnostics,
			sender
				? `Nothing runs "${label(node)}"; the file starts it with a message from "${sender}", which is not execution`
				: `Nothing runs "${label(node)}"; it has no incoming flow`,
			node.id,
			node.name,
		);
	}
}

/**
 * A pool drawn without a start event, given one. Descriptive collaborations
 * routinely leave a participant's process to be started by a message flow from
 * another pool; without an entry node nothing in it could ever run.
 */
function ensureProcessEntry(
	ctx: Ctx,
	process: BpmnProcess,
	scope: Scope,
): void {
	if (process.flowNodes.length === 0) return;
	if (process.flowNodes.some((node) => node.kind === "startEvent")) return;

	const heads = process.flowNodes.filter(
		(node) => node.incoming.length === 0 && node.kind !== "boundaryEvent",
	);
	if (heads.length === 0) return;

	const name = process.participantName ?? process.name ?? process.id;
	const position = processPosition(ctx, process);
	const start = placeOrThrow(ctx, "events_generic", {
		x: position.x - STEP_X,
		y: position.y,
		layer: scope.layerId,
		friendlyName: name,
		comment: `Entry for "${name}", which BPMN draws without a start event\nConfigure what triggers it`,
		start: true,
	});
	ctx.stats.directMapped += 1;
	warn(
		ctx.diagnostics,
		`Pool "${name}" has no start event; added one so it can run — the file starts it through a message flow`,
	);

	const from = port(start, "exec_out", IPinType.Output);
	if (heads.length === 1) {
		const target = resolveEntry(ctx, heads[0].id, undefined);
		if (target) connect(ctx, from, target);
		return;
	}

	const fan = placeOrThrow(ctx, "control_sequence", {
		x: position.x - STEP_X / 2,
		y: position.y,
		layer: scope.layerId,
		comment: `Enter "${name}" at its ${heads.length} unconnected elements`,
	});
	connect(ctx, from, port(fan, "exec_in", IPinType.Input));
	const outs = ensurePins(
		fan,
		"exec_out",
		IPinType.Output,
		Math.max(2, heads.length),
	);
	heads.forEach((head, index) => {
		const target = resolveEntry(ctx, head.id, undefined);
		if (target) connect(ctx, { node: fan, pin: outs[index] }, target);
	});
}

// ───────────────────────────── flow nodes ─────────────────────────────

function translateFlowNode(ctx: Ctx, node: BpmnFlowNode, scope: Scope): void {
	if (isGatewayKind(node.kind)) {
		translateGateway(ctx, node, scope);
		return;
	}
	if (isEventKind(node.kind)) {
		translateEvent(ctx, node, scope);
		return;
	}
	translateActivity(ctx, node, scope);
}

function primaryDef(node: BpmnFlowNode): BpmnEventDefinition | undefined {
	return node.eventDefinitions[0];
}

/**
 * A timer's wait in milliseconds. Executable BPMN puts it in the definition;
 * a diagram drawn to be read often leaves that empty and names the event
 * "60 minutes", which is then the only statement of the wait in the file.
 */
function timerMs(
	def: BpmnEventDefinition | undefined,
	node: BpmnFlowNode,
): number | undefined {
	const timer = def?.timer;
	if (!timer) return undefined;
	const seconds =
		(timer.timeDuration
			? iso8601DurationToSeconds(timer.timeDuration)
			: timer.timeCycle
				? iso8601CycleDurationToSeconds(timer.timeCycle)
				: undefined) ??
		(node.name ? labelDurationToSeconds(node.name) : undefined);
	return seconds === undefined ? undefined : Math.round(seconds * 1000);
}

function translateEvent(ctx: Ctx, node: BpmnFlowNode, scope: Scope): void {
	const def = primaryDef(node);
	const position = nodePosition(ctx, node.id);
	const anchor = anchorOf(ctx, node.id);
	ctx.stats.totalNodes += 1;

	switch (node.kind) {
		case "startEvent": {
			const embedded = scope.activity && !scope.activity.triggeredByEvent;
			if (embedded || scope.functionLayer) {
				// The enclosing activity's incoming flow (or the function's exec input)
				// is the trigger; the start event itself is transparent.
				anchor.through = node.outgoing;
				if (node.eventDefinitions.length > 0) {
					info(
						ctx.diagnostics,
						`Start event "${label(node)}" inside "${scope.activity ? label(scope.activity) : "called process"}" is entered through the enclosing flow; its ${def?.kind} trigger is not enforced`,
						node.id,
						node.name,
					);
				}
				if (scope.functionLayer) registerFunctionEntry(ctx, scope, node);
				return;
			}
			const isMessage = def?.kind === "message";
			const start = placeOrThrow(
				ctx,
				isMessage ? "events_generic" : "events_simple",
				{
					x: position.x,
					y: position.y,
					layer: scope.layerId,
					friendlyName: label(node),
					comment: describeNode(node),
					start: true,
				},
			);
			if (def && def.kind !== "message") {
				warn(
					ctx.diagnostics,
					`Start event "${label(node)}" is triggered by ${describeEventDef(def)}; configure the matching event (${def.kind === "timer" ? "cron" : def.kind}) on the placed start node`,
					node.id,
					node.name,
				);
			}
			if (scope.activity?.triggeredByEvent) {
				warn(
					ctx.diagnostics,
					`Event sub-process "${label(scope.activity)}" starts independently; scoping to the parent instance is not enforced`,
					node.id,
					node.name,
				);
			}
			ctx.stats.directMapped += 1;
			anchor.defaultExits = [port(start, "exec_out", IPinType.Output)];
			splitOutgoing(ctx, node, anchor, scope, "parallel");
			return;
		}
		case "endEvent": {
			const impl = translateThrow(ctx, node, def, scope, position, true);
			anchor.entry = impl.entry;
			for (const exit of impl.exits) scope.endPorts.push(exit);
			if (scope.functionLayer) registerFunctionExit(ctx, scope, impl.exits);
			return;
		}
		case "intermediateThrowEvent": {
			if (def?.kind === "link") {
				const paired = def.linkName
					? ctx.linkCatches.get(`${scope.processId}::${def.linkName}`)
					: undefined;
				if (paired) {
					anchor.through = paired.outgoing;
					info(
						ctx.diagnostics,
						`Link "${def.linkName}" connects "${label(node)}" to "${label(paired)}"`,
						node.id,
						node.name,
					);
					ctx.stats.directMapped += 1;
					return;
				}
				warn(
					ctx.diagnostics,
					`Link throw event "${label(node)}" has no matching catch event`,
					node.id,
					node.name,
				);
			}
			const impl = translateThrow(ctx, node, def, scope, position, false);
			anchor.entry = impl.entry;
			anchor.defaultExits = impl.exits;
			splitOutgoing(ctx, node, anchor, scope, "parallel");
			return;
		}
		case "intermediateCatchEvent": {
			if (def?.kind === "link") {
				anchor.through = node.outgoing;
				ctx.stats.directMapped += 1;
				return;
			}
			const ms = timerMs(def, node);
			if (def?.kind === "timer") {
				// A timer catch is a wait whatever the file says about how long, so
				// the node is right even when only its duration has to be filled in.
				const delay = placeOrThrow(ctx, "delay", {
					x: position.x,
					y: position.y,
					layer: scope.layerId,
					comment: `${label(node)}\n${describeEventDef(def)}${ms === undefined ? "\nSet how long to wait" : ""}`,
				});
				if (ms === undefined) {
					warn(
						ctx.diagnostics,
						`Timer "${label(node)}" does not say how long to wait; set the delay by hand`,
						node.id,
						node.name,
					);
				} else {
					setDefault(delay, "time", ms);
				}
				ctx.stats.directMapped += 1;
				anchor.entry = port(delay, "exec_in", IPinType.Input);
				anchor.defaultExits = [port(delay, "exec_out", IPinType.Output)];
			} else {
				const impl = placeholder(ctx, node, scope, position, catchHint(def));
				anchor.entry = impl.entry;
				anchor.defaultExits = impl.exits;
			}
			splitOutgoing(ctx, node, anchor, scope, "parallel");
			return;
		}
		case "boundaryEvent": {
			// Attached while its host is translated; this only catches an orphan.
			if (!node.attachedToRef || !ctx.nodesById.has(node.attachedToRef)) {
				const impl = placeholder(ctx, node, scope, position);
				anchor.entry = impl.entry;
				anchor.defaultExits = impl.exits;
				warn(
					ctx.diagnostics,
					`Boundary event "${label(node)}" is not attached to a known activity`,
					node.id,
					node.name,
				);
				splitOutgoing(ctx, node, anchor, scope, "parallel");
			}
			return;
		}
		default:
			return;
	}
}

function catchHint(def: BpmnEventDefinition | undefined): string | undefined {
	if (def?.kind === "message" || def?.kind === "signal") {
		return "Model the wait with a second event entry (Generic Event) or a Call Remote Event with wait_for_result";
	}
	if (def?.kind === "timer") {
		return "Only ISO 8601 durations map to a Delay node; dates and cycles need a cron event";
	}
	return undefined;
}

/** End and intermediate throw events: what happens when the token arrives. */
function translateThrow(
	ctx: Ctx,
	node: BpmnFlowNode,
	def: BpmnEventDefinition | undefined,
	scope: Scope,
	position: { x: number; y: number },
	isEnd: boolean,
): Implementation {
	const name = label(node);
	const kind = def?.kind;
	const detail = describeNode(node);

	if (kind === "error") {
		const catcher = findBoundaryCatcher(ctx, scope, "error", def);
		if (catcher) {
			const impl = placeLog(ctx, "log_error", scope, position, {
				comment: `${name}\n${detail}`,
				message: `Error: ${def?.refName ?? def?.code ?? name}`,
			});
			ctx.stats.directMapped += 1;
			for (const exit of impl.exits) connect(ctx, exit, catcher);
			info(
				ctx.diagnostics,
				`Error thrown by "${name}" is routed to its boundary catch event`,
				node.id,
				node.name,
			);
			return { entry: impl.entry, exits: [] };
		}
		// Nothing catches it, so the run should fail here rather than continue as
		// if the error had not happened. `flow_assert` is the only halting node.
		const assert = placeOrThrow(ctx, "flow_assert", {
			x: position.x,
			y: position.y,
			layer: scope.layerId,
			comment: `${name}\nFails the run with this error\n${detail}`,
		});
		setDefault(assert, "condition", false);
		setDefault(assert, "label", def?.code ?? def?.refName ?? name);
		setDefault(assert, "details", describeEventDef(def as BpmnEventDefinition));
		ctx.stats.directMapped += 1;
		return { entry: port(assert, "exec_in", IPinType.Input), exits: [] };
	}

	if (kind === "escalation") {
		const catcher = findBoundaryCatcher(ctx, scope, "escalation", def);
		const impl = placeLog(ctx, "log_warning", scope, position, {
			comment: `${name}\n${detail}`,
			message: `Escalation: ${def?.refName ?? def?.code ?? name}`,
			terminal: isEnd && !catcher,
		});
		ctx.stats.directMapped += 1;
		if (catcher) {
			for (const exit of impl.exits) connect(ctx, exit, catcher);
			return { entry: impl.entry, exits: [] };
		}
		return impl;
	}

	if (kind === "message") {
		if (implementationHint(node) === "email") {
			return sendComposition(ctx, node, scope, position, def);
		}
		return placeholder(
			ctx,
			node,
			scope,
			position,
			`Sends message "${def?.refName ?? def?.ref ?? name}". Candidates: email_smtp_send, notify_user, call_remote_event`,
		);
	}

	if (kind === "terminate") {
		warn(
			ctx.diagnostics,
			`Terminate end event "${name}" cannot stop parallel branches; it only logs`,
			node.id,
			node.name,
		);
		ctx.stats.directMapped += 1;
		return placeLog(ctx, "log_warning", scope, position, {
			comment: `${name}\nTerminate end event: flow-like has no abort node, other branches keep running\n${detail}`,
			message: `Terminate: ${name}`,
			terminal: true,
		});
	}

	if (kind === "signal" || kind === "compensate" || kind === "cancel") {
		warn(
			ctx.diagnostics,
			`${humanKind(kind)} event "${name}" has no runtime equivalent; it logs and continues`,
			node.id,
			node.name,
		);
		ctx.stats.directMapped += 1;
		return placeLog(ctx, "log_warning", scope, position, {
			comment: `${name}\n${detail}`,
			message: `${humanKind(kind)}: ${def?.refName ?? name}`,
			terminal: isEnd,
		});
	}

	ctx.stats.directMapped += 1;
	return placeLog(ctx, "log_info", scope, position, {
		comment: `${name}\n${detail}`,
		message: `${isEnd ? "End" : "Milestone"}: ${name}`,
	});
}

/**
 * Walks up the scope chain for an enclosing activity with a boundary event of
 * `kind` that matches the thrown definition (or catches everything).
 */
function findBoundaryCatcher(
	ctx: Ctx,
	scope: Scope,
	kind: "error" | "escalation",
	def: BpmnEventDefinition | undefined,
): ExecPort | undefined {
	for (
		let current: Scope | undefined = scope;
		current;
		current = current.parent
	) {
		const activity = current.activity;
		if (!activity) continue;
		const candidates = (ctx.boundaries.get(activity.id) ?? []).filter((b) =>
			b.eventDefinitions.some(
				(d) =>
					d.kind === kind &&
					(!d.ref || !def?.ref || d.ref === def.ref) &&
					(!d.code || !def?.code || d.code === def.code),
			),
		);
		const specific = candidates.find((b) =>
			b.eventDefinitions.some(
				(d) => d.kind === kind && d.ref && d.ref === def?.ref,
			),
		);
		const chosen = specific ?? candidates[0];
		const anchor = chosen ? ctx.anchors.get(chosen.id) : undefined;
		if (anchor?.entry) return anchor.entry;
	}
	return undefined;
}

// ───────────────────────────── activities ─────────────────────────────

type ImplementationHint = "http" | "email" | "ai" | undefined;

/**
 * What a task actually does, when the file says so. Vendor extensions are
 * trusted (connector ids, job types, delegate class names); the element name
 * only counts for unambiguous words, so "Request approval" stays a placeholder.
 */
function implementationHint(node: BpmnFlowNode): ImplementationHint {
	const connectorId = findExtension(
		node,
		"camunda",
		"connector",
	)?.element.children.find((c) => c.name === "connectorId")?.text;
	const vendor = [
		findExtension(node, "zeebe", "taskDefinition")?.attrs.type,
		node.vendorAttrs["zeebe:modelerTemplate"],
		node.vendorAttrs["camunda:type"],
		node.vendorAttrs["camunda:topic"],
		node.vendorAttrs["camunda:class"],
		node.vendorAttrs["camunda:delegateExpression"],
		node.vendorAttrs["camunda:expression"],
		node.vendorAttrs["flowable:type"],
		node.vendorAttrs["activiti:type"],
		connectorId,
	]
		.filter((value): value is string => Boolean(value))
		.join(" ")
		.toLowerCase();
	if (/openai|anthropic|claude|gemini|llm|gpt|mistral|\bai\b/.test(vendor)) {
		return "ai";
	}
	if (/e-?mail|smtp|sendgrid|mailgun|\bses\b|\bmail\b/.test(vendor)) {
		return "email";
	}
	if (/http|https|rest|webhook|\bapi\b|fetch/.test(vendor)) return "http";

	const name = (node.name ?? "").toLowerCase();
	if (node.kind === "serviceTask" || node.kind === "sendTask") {
		if (/\b(e-?mail|smtp)\b/.test(name)) return "email";
	}
	if (node.kind === "serviceTask") {
		if (/\b(llm|gpt|chatgpt|openai|claude|gemini)\b/.test(name)) return "ai";
		if (/\b(http|rest|webhook|api)\b/.test(name)) return "http";
	}
	return undefined;
}

/** What to suggest for a task kind that has no equivalent at all. */
const ACTIVITY_HINTS: Partial<Record<BpmnFlowNode["kind"], string>> = {
	sendTask:
		"Candidates: email_smtp_send, notify_user, telegram_send_message, discord_send_message",
	serviceTask:
		"Candidates: http_fetch (REST), call_remote_event (another project), ai_generative_invoke_simple (LLM)",
	scriptTask:
		"No script runtime in the catalog; rebuild the logic with nodes (eval for arithmetic, string_render_template for text)",
	userTask:
		"Human step: in chat flows use interaction_single_choice / interaction_form; in pages use a widget action event",
	manualTask:
		"Human step: in chat flows use interaction_single_choice / interaction_form; in pages use a widget action event",
	businessRuleTask:
		"Decision: model with control_switch or a chain of control_branch nodes",
	receiveTask:
		"Waiting for a message: split the flow at this point into a second event entry (Generic Event)",
};

function translateActivity(ctx: Ctx, node: BpmnFlowNode, scope: Scope): void {
	const position = nodePosition(ctx, node.id);
	const anchor = anchorOf(ctx, node.id);
	ctx.stats.totalNodes += 1;
	const markers = createBoundaryMarkers(ctx, node, scope, position);

	let impl = translateActivityBody(ctx, node, scope, position);
	if (!impl) return;

	impl = attachDataAssociations(ctx, node, scope, position, impl);
	impl = attachBoundaries(ctx, node, scope, position, impl, markers);
	impl = attachLoop(ctx, node, scope, position, impl);

	anchor.entry = impl.entry;
	anchor.defaultExits = impl.exits;
	splitOutgoing(ctx, node, anchor, scope, "inclusive");
}

/** The activity itself, before boundaries, loops and data associations wrap it. */
function translateActivityBody(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
): Implementation | undefined {
	switch (node.kind) {
		case "subProcess":
		case "adHocSubProcess":
		case "transaction":
			return translateSubProcess(ctx, node, scope, position);
		case "callActivity":
			return translateCallActivity(ctx, node, scope, position);
		case "serviceTask":
		case "sendTask":
		case "task":
			switch (implementationHint(node)) {
				case "http":
					return httpComposition(ctx, node, scope, position);
				case "email":
					return sendComposition(ctx, node, scope, position);
				case "ai":
					return aiComposition(ctx, node, scope, position);
				default:
					return placeholder(
						ctx,
						node,
						scope,
						position,
						ACTIVITY_HINTS[node.kind],
					);
			}
		default:
			return placeholder(ctx, node, scope, position, ACTIVITY_HINTS[node.kind]);
	}
}

/** Collapsed layer named after the element, with a warning log inside so it runs and shows up in FlowScript. */
function placeholder(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
	hint?: string,
): Implementation {
	const name = label(node);
	const layer = createLayer(ctx, {
		name,
		comment: `TODO: model this ${humanKind(node.kind).toLowerCase()}\n${describeNode(node, hint ? [hint] : [])}`,
		color: PLACEHOLDER_COLOR,
		parentId: scope.layerId,
		position,
	});
	const impl = placeLog(ctx, "log_warning", scope, position, {
		layer: layer.id,
		comment: `${name}\nReplace this with the real implementation`,
		message: `Not implemented: ${humanKind(node.kind)} "${name}"`,
	});
	ctx.stats.todo += 1;
	warn(
		ctx.diagnostics,
		`${humanKind(node.kind)} "${name}" has no direct equivalent; created a placeholder layer${hint ? `. ${hint}` : ""}`,
		node.id,
		node.name,
	);
	return impl;
}

function compositionLayer(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
	what: string,
): ILayer {
	return createLayer(ctx, {
		name: label(node),
		comment: `${what}\n${describeNode(node)}`,
		color: COMPOSITION_COLOR,
		parentId: scope.layerId,
		position,
	});
}

/** An io-mapping input by name, with the expression markers a caller cannot use stripped. */
function ioValue(node: BpmnFlowNode, key: string): string | undefined {
	const match = node.ioMapping?.inputs.find(
		(io) => io.target.toLowerCase() === key.toLowerCase(),
	);
	const value = match?.source;
	if (!value) return undefined;
	return value.replace(/^=/, "").replace(/^"(.*)"$/, "$1");
}

/** Records a composition against the element it came from. */
function composed(
	ctx: Ctx,
	node: BpmnFlowNode,
	what: string,
	impl: Implementation,
): Implementation {
	ctx.stats.composed += 1;
	info(
		ctx.diagnostics,
		`"${label(node)}" mapped to ${what}`,
		node.id,
		node.name,
	);
	return impl;
}

function httpComposition(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
): Implementation {
	if (!catalogHas(ctx.catalog, "http_fetch")) {
		return placeholder(ctx, node, scope, position, ACTIVITY_HINTS.serviceTask);
	}
	const layer = compositionLayer(ctx, node, scope, position, "HTTP request");
	const make = placeOrThrow(ctx, "http_make_request", {
		x: position.x - STEP_X,
		y: position.y + STEP_Y / 2,
		layer: layer.id,
		comment: label(node),
	});
	const url = ioValue(node, "url");
	const method = ioValue(node, "method");
	if (url) setDefault(make, "url", url);
	if (method) setDefault(make, "method", method.toUpperCase());
	const fetch = placeOrThrow(ctx, "http_fetch", {
		x: position.x,
		y: position.y,
		layer: layer.id,
		comment: label(node),
	});
	const toJson = placeOrThrow(ctx, "http_response_to_json", {
		x: position.x + STEP_X,
		y: position.y,
		layer: layer.id,
	});
	connectData(make, "request", fetch, "request");
	connect(
		ctx,
		port(fetch, "exec_success", IPinType.Output),
		port(toJson, "exec_in", IPinType.Input),
	);
	connectData(fetch, "response", toJson, "response");
	return composed(ctx, node, "an HTTP request composition", {
		entry: port(fetch, "exec_in", IPinType.Input),
		exits: [port(toJson, "exec_out", IPinType.Output)],
		errorExit: port(fetch, "exec_error", IPinType.Output),
	});
}

function sendComposition(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
	def?: BpmnEventDefinition,
): Implementation {
	if (!catalogHas(ctx.catalog, "email_smtp_send")) {
		return placeholder(ctx, node, scope, position, ACTIVITY_HINTS.sendTask);
	}
	const layer = compositionLayer(
		ctx,
		node,
		scope,
		position,
		"Send message (SMTP)",
	);
	const connectNode = placeOrThrow(ctx, "email_smtp_connect", {
		x: position.x - STEP_X,
		y: position.y,
		layer: layer.id,
		comment: "Configure the SMTP server",
	});
	const send = placeOrThrow(ctx, "email_smtp_send", {
		x: position.x,
		y: position.y,
		layer: layer.id,
		comment: label(node),
	});
	setDefault(send, "subject", def?.refName ?? label(node));
	const to = ioValue(node, "to") ?? ioValue(node, "recipient");
	if (to) setDefault(send, "to", to);
	connect(
		ctx,
		port(connectNode, "exec_out", IPinType.Output),
		port(send, "exec_in", IPinType.Input),
	);
	connectData(connectNode, "connection", send, "connection");
	return composed(ctx, node, "an SMTP send composition", {
		entry: port(connectNode, "exec_in", IPinType.Input),
		exits: [port(send, "exec_out", IPinType.Output)],
	});
}

function aiComposition(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
): Implementation {
	if (!catalogHas(ctx.catalog, "ai_generative_invoke_simple")) {
		return placeholder(ctx, node, scope, position, ACTIVITY_HINTS.serviceTask);
	}
	const layer = compositionLayer(ctx, node, scope, position, "LLM call");
	const find = placeOrThrow(ctx, "ai_generative_find_model", {
		x: position.x - STEP_X,
		y: position.y,
		layer: layer.id,
	});
	const invoke = placeOrThrow(ctx, "ai_generative_invoke_simple", {
		x: position.x,
		y: position.y,
		layer: layer.id,
		comment: label(node),
	});
	setDefault(invoke, "stream", false);
	const prompt = ioValue(node, "prompt") ?? node.documentation;
	if (prompt) setDefault(invoke, "prompt", prompt);
	const system =
		ioValue(node, "systemPrompt") ?? ioValue(node, "system_prompt");
	if (system) setDefault(invoke, "system_prompt", system);
	connect(
		ctx,
		port(find, "exec_out", IPinType.Output),
		port(invoke, "exec_in", IPinType.Input),
	);
	connectData(find, "model", invoke, "model");
	return composed(ctx, node, "an LLM composition", {
		entry: port(find, "exec_in", IPinType.Input),
		exits: [port(invoke, "done", IPinType.Output)],
	});
}

const EMPTY_CONTAINER: BpmnContainer = {
	flowNodes: [],
	sequenceFlows: [],
	dataObjects: [],
	annotations: [],
	associations: [],
	groups: [],
	lanes: [],
};

function subProcessNote(node: BpmnFlowNode): string {
	if (node.triggeredByEvent) return "Event sub-process";
	if (node.kind === "adHocSubProcess") {
		return "Ad-hoc sub-process: inner activities have no prescribed order";
	}
	if (node.kind === "transaction") {
		return "Transaction: compensation is not modelled";
	}
	return "Sub-process";
}

function translateSubProcess(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
): Implementation | undefined {
	const body = node.body ?? EMPTY_CONTAINER;
	const layer = createLayer(ctx, {
		name: label(node),
		comment: describeNode(node, [subProcessNote(node)]),
		color: SCOPE_COLOR,
		parentId: scope.layerId,
		position,
	});
	const inner: Scope = {
		layerId: layer.id,
		processId: scope.processId,
		activity: node,
		parent: scope,
		endPorts: [],
	};
	translateContainer(ctx, body, inner);
	translateLanes(ctx, body.lanes, inner);
	ctx.stats.directMapped += 1;

	if (node.triggeredByEvent) {
		// Its start events are independent entries; nothing flows in or out.
		return undefined;
	}

	const entry = subProcessEntry(ctx, node, body, layer.id, position);
	if (inner.endPorts.length === 0 && body.flowNodes.length > 0) {
		warn(
			ctx.diagnostics,
			`Sub-process "${label(node)}" has no end event; nothing continues after it`,
			node.id,
			node.name,
		);
	}
	return { entry, exits: inner.endPorts };
}

/**
 * Where the enclosing flow enters a sub-process: straight at the first element
 * when there is one start event, through a fork when there are several, and
 * through a sequence over the unconnected activities when there is none.
 */
function subProcessEntry(
	ctx: Ctx,
	node: BpmnFlowNode,
	body: BpmnContainer,
	layerId: string,
	position: { x: number; y: number },
): ExecPort {
	const starts = body.flowNodes.filter((n) => n.kind === "startEvent");
	const startFlows = starts.flatMap((start) => start.outgoing);

	if (startFlows.length === 1) {
		const flow = ctx.flowsById.get(startFlows[0]);
		const target = flow
			? resolveEntry(ctx, flow.targetRef, flow.id)
			: undefined;
		if (target) return target;
	}

	if (startFlows.length > 1) {
		const fan = placeOrThrow(ctx, "control_par_execution", {
			x: position.x - STEP_X,
			y: position.y,
			layer: layerId,
			comment: `Enter ${label(node)} (${starts.length} start events)`,
		});
		const outs = ensurePins(
			fan,
			"exec_out",
			IPinType.Output,
			Math.max(2, startFlows.length),
		);
		startFlows.forEach((flowId, index) => {
			const flow = ctx.flowsById.get(flowId);
			const target = flow
				? resolveEntry(ctx, flow.targetRef, flowId)
				: undefined;
			if (target) connect(ctx, { node: fan, pin: outs[index] }, target);
		});
		return port(fan, "exec_in", IPinType.Input);
	}

	const heads = body.flowNodes.filter(
		(n) =>
			n.incoming.length === 0 &&
			n.kind !== "boundaryEvent" &&
			n.kind !== "startEvent",
	);
	const fan = placeOrThrow(ctx, "control_sequence", {
		x: position.x - STEP_X,
		y: position.y,
		layer: layerId,
		comment: `Enter ${label(node)}`,
	});
	const outs = ensurePins(
		fan,
		"exec_out",
		IPinType.Output,
		Math.max(2, heads.length),
	);
	heads.forEach((head, index) => {
		const target = resolveEntry(ctx, head.id, undefined);
		if (target) connect(ctx, { node: fan, pin: outs[index] }, target);
	});
	if (starts.length === 0 && body.flowNodes.length > 0) {
		warn(
			ctx.diagnostics,
			`Sub-process "${label(node)}" has no start event; ${node.kind === "adHocSubProcess" ? "its activities run in sequence" : "entering at the activities without incoming flows"}`,
			node.id,
			node.name,
		);
	}
	return port(fan, "exec_in", IPinType.Input);
}

function translateCallActivity(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
): Implementation {
	const fn = node.calledElement
		? ctx.functionByProcess.get(node.calledElement)
		: undefined;
	if (!fn || !catalogHas(ctx.catalog, "control_call_function")) {
		return placeholder(
			ctx,
			node,
			scope,
			position,
			node.calledElement
				? `Calls process "${node.calledElement}", which is not in this file; use control_call_function or call_remote_event`
				: "No called element; use control_call_function or call_remote_event",
		);
	}
	const call = placeOrThrow(ctx, "control_call_function", {
		x: position.x,
		y: position.y,
		layer: scope.layerId,
		comment: `${label(node)}\nCalls ${fn.name}\n${describeNode(node)}`,
		friendlyName: `Call ${fn.name}`,
	});
	setDefault(call, "function_layer_id", fn.id);
	// `on_update` will mirror the function's pins by name once the board loads;
	// minting them here is what lets the sequence flow be wired straight away.
	for (const pin of Object.values(fn.pins)) {
		mintPin(call, {
			name: pin.name,
			friendly: pin.friendly_name,
			type: pin.pin_type as IPinType,
			data: pin.data_type as IVariableType,
		});
	}
	ctx.stats.directMapped += 1;
	info(
		ctx.diagnostics,
		`Call activity "${label(node)}" calls function "${fn.name}"`,
		node.id,
		node.name,
	);
	return {
		entry: port(call, "exec_in", IPinType.Input),
		exits: [port(call, "exec_out", IPinType.Output)],
	};
}

function functionBoundaryPin(scope: Scope, type: IPinType) {
	return Object.values(scope.functionLayer?.pins ?? {}).find(
		(pin) => pin.pin_type === type && pin.data_type === IVariableType.Execution,
	);
}

/**
 * The function layer's exec input stands in for the called process's start
 * event; what it connects to is only known once the whole fragment exists.
 */
function registerFunctionEntry(
	ctx: Ctx,
	scope: Scope,
	start: BpmnFlowNode,
): void {
	const execIn = functionBoundaryPin(scope, IPinType.Input);
	if (!execIn) return;
	for (const flowId of start.outgoing) {
		const flow = ctx.flowsById.get(flowId);
		if (!flow) continue;
		ctx.pendingFunctionEntries.push({
			pin: execIn,
			targetId: flow.targetRef,
			flowId,
		});
	}
}

function registerFunctionExit(ctx: Ctx, scope: Scope, exits: ExecPort[]): void {
	const execOut = functionBoundaryPin(scope, IPinType.Output);
	if (!execOut) return;
	for (const exit of exits) {
		exit.pin.connected_to = [execOut.id];
		if (!execOut.depends_on.includes(exit.pin.id)) {
			execOut.depends_on.push(exit.pin.id);
		}
		ctx.stats.connections += 1;
	}
}

// ───────────────────────────── wrappers ─────────────────────────────

/**
 * Data objects an activity reads and writes, as variable nodes beside it. The
 * read is left unwired — which of the activity's inputs wants it is a modelling
 * decision — while the write is spliced into the exec chain after it.
 */
function attachDataAssociations(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
	impl: Implementation,
): Implementation {
	let result = impl;
	node.dataInputs.forEach((dataId, index) => {
		const variable = ctx.variablesByData.get(dataId);
		if (!variable || !catalogHas(ctx.catalog, "variable_get")) return;
		const get = placeOrThrow(ctx, "variable_get", {
			x: position.x - STEP_X / 2,
			y: position.y - STEP_Y * (index + 1),
			layer: scope.layerId,
			comment: `${label(node)} reads ${variable.name}`,
		});
		setDefault(get, "var_ref", variable.id);
		info(
			ctx.diagnostics,
			`"${label(node)}" reads data object "${variable.name}"`,
			node.id,
			node.name,
		);
	});
	node.dataOutputs.forEach((dataId, index) => {
		const variable = ctx.variablesByData.get(dataId);
		if (!variable || !catalogHas(ctx.catalog, "variable_set")) return;
		const set = placeOrThrow(ctx, "variable_set", {
			x: position.x + STEP_X / 2,
			y: position.y + STEP_Y * (index + 1),
			layer: scope.layerId,
			comment: `${label(node)} writes ${variable.name}\nConnect the value to store`,
		});
		setDefault(set, "var_ref", variable.id);
		const setIn = port(set, "exec_in", IPinType.Input);
		for (const exit of result.exits) connect(ctx, exit, setIn);
		result = { ...result, exits: [port(set, "exec_out", IPinType.Output)] };
		info(
			ctx.diagnostics,
			`"${label(node)}" writes data object "${variable.name}"`,
			node.id,
			node.name,
		);
	});
	return result;
}

interface BoundaryMarker {
	boundary: BpmnFlowNode;
	def: BpmnEventDefinition | undefined;
	marker: INode;
	position: { x: number; y: number };
}

/**
 * Every boundary event becomes a log marker whose exec input is the catch
 * point. Created before the host is translated so throw events inside a
 * sub-process can already route to it.
 */
function createBoundaryMarkers(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
): BoundaryMarker[] {
	return (ctx.boundaries.get(node.id) ?? []).map((boundary, index) => {
		const def = primaryDef(boundary);
		const bPosition = positionOf(ctx, boundary.id) ?? {
			x: position.x + STEP_X / 2,
			y: position.y + STEP_Y * (index + 1),
		};
		ctx.stats.totalNodes += 1;
		const impl = placeLog(ctx, "log_info", scope, bPosition, {
			comment: `${label(boundary)}\n${boundary.interrupting ? "Interrupting" : "Non-interrupting"} boundary event on "${label(node)}"\n${describeNode(boundary)}`,
			message: `Boundary event: ${label(boundary)}`,
		});
		ctx.stats.directMapped += 1;
		const anchor = anchorOf(ctx, boundary.id);
		anchor.entry = impl.entry;
		anchor.defaultExits = impl.exits;
		return { boundary, def, marker: impl.entry.node, position: bPosition };
	});
}

function attachBoundaries(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
	impl: Implementation,
	markers: BoundaryMarker[],
): Implementation {
	let result = impl;
	let errorUsed = false;
	for (const { boundary, def, marker, position: bPosition } of markers) {
		const markerIn = port(marker, "exec_in", IPinType.Input);
		const ms = timerMs(def, boundary);
		if (def?.kind === "timer" && ms !== undefined && boundary.interrupting) {
			// Interrupting: the host runs inside the timeout, so expiry cancels it.
			const timeout = placeOrThrow(ctx, "control_timeout", {
				x: position.x - STEP_X,
				y: position.y - STEP_Y,
				layer: scope.layerId,
				comment: `Timer boundary "${label(boundary)}" on "${label(node)}"\n${describeEventDef(def)}`,
			});
			setDefault(timeout, "timeout_ms", ms);
			connect(ctx, port(timeout, "exec_body", IPinType.Output), result.entry);
			connect(ctx, port(timeout, "exec_timed_out", IPinType.Output), markerIn);
			result = {
				entry: port(timeout, "exec_in", IPinType.Input),
				exits: [port(timeout, "exec_completed", IPinType.Output)],
				errorExit: result.errorExit,
			};
			info(
				ctx.diagnostics,
				`"${label(node)}" runs under a ${ms} ms timeout (boundary "${label(boundary)}")`,
				boundary.id,
				boundary.name,
			);
		} else if (def?.kind === "timer" && ms !== undefined) {
			// Non-interrupting: the host keeps running, so the timer is a branch.
			const fork = placeOrThrow(ctx, "control_par_execution", {
				x: position.x - STEP_X,
				y: position.y - STEP_Y,
				layer: scope.layerId,
				comment: `Non-interrupting timer "${label(boundary)}" on "${label(node)}"`,
			});
			const delay = placeOrThrow(ctx, "delay", {
				x: bPosition.x - STEP_X / 2,
				y: bPosition.y,
				layer: scope.layerId,
				comment: describeEventDef(def),
			});
			setDefault(delay, "time", ms);
			const outs = ensurePins(fork, "exec_out", IPinType.Output, 2);
			connect(ctx, { node: fork, pin: outs[0] }, result.entry);
			connect(
				ctx,
				{ node: fork, pin: outs[1] },
				port(delay, "exec_in", IPinType.Input),
			);
			connect(ctx, port(delay, "exec_out", IPinType.Output), markerIn);
			result = { ...result, entry: port(fork, "exec_in", IPinType.Input) };
			warn(
				ctx.diagnostics,
				`Non-interrupting timer "${label(boundary)}" fires after ${ms} ms even if "${label(node)}" finished earlier`,
				boundary.id,
				boundary.name,
			);
		} else if (def?.kind === "error" && result.errorExit && !errorUsed) {
			connect(ctx, result.errorExit, markerIn);
			errorUsed = true;
			info(
				ctx.diagnostics,
				`Error boundary "${label(boundary)}" is wired to the error output of "${label(node)}"`,
				boundary.id,
				boundary.name,
			);
		} else if (def?.kind === "error" || def?.kind === "escalation") {
			// A throw event inside the host may already have routed to this marker.
			if (markerIn.pin.depends_on.length === 0) {
				warn(
					ctx.diagnostics,
					`${humanKind(def.kind)} boundary "${label(boundary)}" on "${label(node)}" has no throw event routed to it; connect it to the implementation's error output`,
					boundary.id,
					boundary.name,
				);
			}
		} else {
			warn(
				ctx.diagnostics,
				`Boundary event "${label(boundary)}" (${def ? describeEventDef(def) : "no trigger"}) on "${label(node)}" needs a manual trigger connection`,
				boundary.id,
				boundary.name,
			);
		}
		splitOutgoing(ctx, boundary, anchorOf(ctx, boundary.id), scope, "parallel");
	}
	return result;
}

function attachLoop(
	ctx: Ctx,
	node: BpmnFlowNode,
	scope: Scope,
	position: { x: number; y: number },
	impl: Implementation,
): Implementation {
	const loop = node.loop;
	if (!loop) return impl;

	if (loop.kind === "multiInstance") {
		const name = loop.isSequential
			? "control_for_each"
			: "control_par_for_each";
		if (!catalogHas(ctx.catalog, name)) return impl;
		const each = placeOrThrow(ctx, name, {
			x: position.x - STEP_X,
			y: position.y + STEP_Y,
			layer: scope.layerId,
			comment: `${loop.isSequential ? "Sequential" : "Parallel"} multi-instance "${label(node)}"\ncollection: ${loop.inputCollection ?? loop.cardinality ?? "?"}${loop.inputElement ? `\nelement: ${loop.inputElement}` : ""}${loop.completionCondition ? `\ncompletion: ${loop.completionCondition}` : ""}\nConnect the array to iterate`,
		});
		connect(ctx, port(each, "exec_out", IPinType.Output), impl.entry);
		info(
			ctx.diagnostics,
			`"${label(node)}" is multi-instance; wrapped in ${loop.isSequential ? "For Each" : "Parallel For Each"}`,
			node.id,
			node.name,
		);
		return {
			entry: port(each, "exec_in", IPinType.Input),
			exits: [port(each, "done", IPinType.Output)],
			errorExit: impl.errorExit,
		};
	}

	if (!catalogHas(ctx.catalog, "control_while_loop")) return impl;
	const loopNode = placeOrThrow(ctx, "control_while_loop", {
		x: position.x - STEP_X,
		y: position.y + STEP_Y,
		layer: scope.layerId,
		comment: `Loop "${label(node)}"\ncondition: ${loop.loopCondition ?? "?"}${loop.loopMaximum ? `\nmax: ${loop.loopMaximum}` : ""}\nWire the loop condition`,
	});
	if (loop.loopMaximum && /^\d+$/.test(loop.loopMaximum)) {
		setDefault(loopNode, "max_iter", Number.parseInt(loop.loopMaximum, 10));
	}
	connect(ctx, port(loopNode, "exec_out", IPinType.Output), impl.entry);
	info(
		ctx.diagnostics,
		`"${label(node)}" has standard loop characteristics; wrapped in While Loop`,
		node.id,
		node.name,
	);
	return {
		entry: port(loopNode, "exec_in", IPinType.Input),
		exits: [port(loopNode, "done", IPinType.Output)],
		errorExit: impl.errorExit,
	};
}

// ───────────────────────────── collaboration ─────────────────────────────

/**
 * What a collaboration adds around the processes: annotations and groups drawn
 * outside any pool, and the message flows between pools, which are not
 * execution and so are recorded rather than wired.
 */
function translateCollaborationArtifacts(ctx: Ctx): void {
	const collab = ctx.defs.collaboration;
	if (!collab) return;

	addArtifactComments(
		ctx,
		{ ...collab, associations: collab.associations },
		null,
		{
			x: 0,
			y: -STEP_Y * 2,
		},
	);

	if (collab.messageFlows.length > 0) {
		const nameOf = (id: string) => {
			const participant = collab.participants.find((p) => p.id === id);
			const node = ctx.nodesById.get(id);
			return participant?.name ?? (node ? label(node) : id);
		};
		const lines = collab.messageFlows.map(
			(mf) =>
				`${nameOf(mf.sourceRef)} → ${nameOf(mf.targetRef)}${mf.name ? `: ${mf.name}` : ""}${mf.messageRef ? ` (${ctx.defs.messages.get(mf.messageRef)?.name ?? mf.messageRef})` : ""}`,
		);
		addComment(ctx, {
			content: `Message flows between pools:\n${lines.join("\n")}`,
			position: { x: 0, y: -STEP_Y * 3 },
			layer: null,
			width: 420,
			height: 40 + lines.length * 20,
		});
		info(
			ctx.diagnostics,
			`${collab.messageFlows.length} message flow(s) between pools are listed in a comment; connect them with remote events where needed`,
		);
	}

	for (const pool of collab.participants.filter((p) => !p.processRef)) {
		info(
			ctx.diagnostics,
			`Pool "${pool.name ?? pool.id}" has no process (black box)`,
		);
	}
}
