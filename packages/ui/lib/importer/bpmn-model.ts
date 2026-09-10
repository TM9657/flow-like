import {
	type XmlElement,
	attr,
	childrenNamed,
	firstChild,
	parseXml,
	textOf,
} from "./xml";

/**
 * A typed reading of a BPMN 2.0 document, as close to the XML as is useful.
 *
 * Everything the translator decides on is surfaced here as data — element
 * kinds, event definitions, gateway defaults, loop characteristics, vendor
 * extensions and diagram geometry — so the translator never touches XML and
 * the model can be unit-tested on its own.
 */

export const BPMN_EVENT_KINDS = [
	"startEvent",
	"endEvent",
	"intermediateCatchEvent",
	"intermediateThrowEvent",
	"boundaryEvent",
] as const;

export const BPMN_ACTIVITY_KINDS = [
	"task",
	"userTask",
	"serviceTask",
	"scriptTask",
	"sendTask",
	"receiveTask",
	"manualTask",
	"businessRuleTask",
	"callActivity",
	"subProcess",
	"adHocSubProcess",
	"transaction",
] as const;

export const BPMN_GATEWAY_KINDS = [
	"exclusiveGateway",
	"inclusiveGateway",
	"parallelGateway",
	"complexGateway",
	"eventBasedGateway",
] as const;

export type BpmnEventKind = (typeof BPMN_EVENT_KINDS)[number];
export type BpmnActivityKind = (typeof BPMN_ACTIVITY_KINDS)[number];
export type BpmnGatewayKind = (typeof BPMN_GATEWAY_KINDS)[number];
export type BpmnFlowNodeKind =
	| BpmnEventKind
	| BpmnActivityKind
	| BpmnGatewayKind;

const FLOW_NODE_KINDS: ReadonlySet<string> = new Set<string>([
	...BPMN_EVENT_KINDS,
	...BPMN_ACTIVITY_KINDS,
	...BPMN_GATEWAY_KINDS,
]);

export const BPMN_SUBPROCESS_KINDS: ReadonlySet<string> = new Set([
	"subProcess",
	"adHocSubProcess",
	"transaction",
]);

export type BpmnEventDefinitionKind =
	| "message"
	| "timer"
	| "error"
	| "signal"
	| "escalation"
	| "cancel"
	| "compensate"
	| "conditional"
	| "link"
	| "terminate";

export interface BpmnEventDefinition {
	kind: BpmnEventDefinitionKind;
	/** `messageRef`, `signalRef`, `errorRef`, `escalationRef` or `activityRef`. */
	ref?: string;
	/** Resolved name of the referenced message/signal/error/escalation. */
	refName?: string;
	/** Error code / escalation code, when the referenced definition carries one. */
	code?: string;
	/** Link events pair by this name. */
	linkName?: string;
	timer?: { timeDate?: string; timeCycle?: string; timeDuration?: string };
	condition?: string;
}

export interface BpmnLoopCharacteristics {
	kind: "standard" | "multiInstance";
	isSequential: boolean;
	cardinality?: string;
	completionCondition?: string;
	loopCondition?: string;
	testBefore?: boolean;
	loopMaximum?: string;
	inputCollection?: string;
	inputElement?: string;
	outputCollection?: string;
	outputElement?: string;
}

export interface BpmnIoMapping {
	inputs: Array<{ source: string; target: string }>;
	outputs: Array<{ source: string; target: string }>;
}

/** One `<extensionElements>` child, kept raw plus a flat attribute view. */
export interface BpmnExtension {
	prefix: string | null;
	name: string;
	attrs: Record<string, string>;
	text?: string;
	element: XmlElement;
}

export interface BpmnFlowNode {
	id: string;
	kind: BpmnFlowNodeKind;
	name?: string;
	documentation?: string;
	incoming: string[];
	outgoing: string[];
	eventDefinitions: BpmnEventDefinition[];
	/** Events: `isInterrupting` on starts, `cancelActivity` on boundaries. */
	interrupting: boolean;
	/** Boundary events: the activity they hang on. */
	attachedToRef?: string;
	/** Gateways and activities: the sequence flow taken when nothing matches. */
	defaultFlow?: string;
	/** Event-based gateways: `Exclusive` (default) or `Parallel`. */
	eventGatewayType?: string;
	instantiate?: boolean;
	/** Sub-processes: an event sub-process is started by its own start event. */
	triggeredByEvent?: boolean;
	/** Call activities: the called process id (or `zeebe:calledElement`). */
	calledElement?: string;
	script?: { format?: string; body?: string; resultVariable?: string };
	loop?: BpmnLoopCharacteristics;
	ioMapping?: BpmnIoMapping;
	/** Ids of data objects / stores read and written through data associations. */
	dataInputs: string[];
	dataOutputs: string[];
	isForCompensation: boolean;
	/** Sub-process body. */
	body?: BpmnContainer;
	/** Vendor attributes on the element (`camunda:assignee`, `zeebe:modelerTemplate`...). */
	vendorAttrs: Record<string, string>;
	extensions: BpmnExtension[];
}

export interface BpmnSequenceFlow {
	id: string;
	name?: string;
	sourceRef: string;
	targetRef: string;
	condition?: string;
	conditionLanguage?: string;
	documentation?: string;
}

export interface BpmnDataObject {
	id: string;
	kind: "dataObject" | "dataObjectReference" | "dataStoreReference";
	name?: string;
	/** References point at their `dataObject` / `dataStore`. */
	ref?: string;
	state?: string;
	isCollection: boolean;
	documentation?: string;
}

export interface BpmnTextAnnotation {
	id: string;
	text?: string;
}

export interface BpmnAssociation {
	id: string;
	sourceRef: string;
	targetRef: string;
	direction?: string;
}

export interface BpmnGroup {
	id: string;
	categoryValueRef?: string;
	name?: string;
}

export interface BpmnLane {
	id: string;
	name?: string;
	flowNodeRefs: string[];
	children: BpmnLane[];
}

export interface BpmnContainer {
	flowNodes: BpmnFlowNode[];
	sequenceFlows: BpmnSequenceFlow[];
	dataObjects: BpmnDataObject[];
	annotations: BpmnTextAnnotation[];
	associations: BpmnAssociation[];
	groups: BpmnGroup[];
	lanes: BpmnLane[];
}

export interface BpmnProcess extends BpmnContainer {
	id: string;
	name?: string;
	isExecutable: boolean;
	documentation?: string;
	/** Name of the collaboration participant (pool) that owns this process. */
	participantName?: string;
	vendorAttrs: Record<string, string>;
	extensions: BpmnExtension[];
}

export interface BpmnParticipant {
	id: string;
	name?: string;
	processRef?: string;
}

export interface BpmnMessageFlow {
	id: string;
	name?: string;
	sourceRef: string;
	targetRef: string;
	messageRef?: string;
}

export interface BpmnCollaboration {
	id: string;
	name?: string;
	participants: BpmnParticipant[];
	messageFlows: BpmnMessageFlow[];
	annotations: BpmnTextAnnotation[];
	associations: BpmnAssociation[];
	groups: BpmnGroup[];
}

export interface BpmnBounds {
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface BpmnShape extends BpmnBounds {
	isExpanded?: boolean;
	isHorizontal?: boolean;
}

export interface BpmnDiagram {
	shapes: Map<string, BpmnShape>;
	edges: Map<string, Array<{ x: number; y: number }>>;
}

export interface BpmnRootDefinition {
	id: string;
	name?: string;
	/** `errorCode` or `escalationCode`. */
	code?: string;
}

export interface BpmnDefinitions {
	id?: string;
	name?: string;
	targetNamespace?: string;
	exporter?: string;
	exporterVersion?: string;
	processes: BpmnProcess[];
	collaboration?: BpmnCollaboration;
	messages: Map<string, BpmnRootDefinition>;
	signals: Map<string, BpmnRootDefinition>;
	errors: Map<string, BpmnRootDefinition>;
	escalations: Map<string, BpmnRootDefinition>;
	diagram: BpmnDiagram;
}

export const BPMN_MODEL_NAMESPACE =
	"http://www.omg.org/spec/BPMN/20100524/MODEL";

/** A document is BPMN when its root is `definitions` and it declares the BPMN namespace or a process. */
export function isBpmnDocument(root: XmlElement): boolean {
	if (root.name !== "definitions") return false;
	const declaresNamespace = Object.entries(root.attrs).some(
		([key, value]) =>
			key.startsWith("xmlns") && value.startsWith(BPMN_MODEL_NAMESPACE),
	);
	if (declaresNamespace) return true;
	return root.children.some(
		(child) => child.name === "process" || child.name === "collaboration",
	);
}

/** Parses a BPMN document, or reads one that has already been parsed. */
export function parseBpmn(source: string | XmlElement): BpmnDefinitions {
	const root = typeof source === "string" ? parseXml(source) : source;
	if (!isBpmnDocument(root)) {
		throw new Error(
			`Not a BPMN document: root element is <${root.prefix ? `${root.prefix}:` : ""}${root.name}>`,
		);
	}
	return readDefinitions(root);
}

function readDefinitions(root: XmlElement): BpmnDefinitions {
	const rootDefs = (name: string, codeAttr?: string) => {
		const map = new Map<string, BpmnRootDefinition>();
		for (const el of childrenNamed(root, name)) {
			const id = el.attrs.id;
			if (!id) continue;
			map.set(id, {
				id,
				name: el.attrs.name,
				code: codeAttr ? el.attrs[codeAttr] : undefined,
			});
		}
		return map;
	};

	const definitions: BpmnDefinitions = {
		id: root.attrs.id,
		name: root.attrs.name,
		targetNamespace: root.attrs.targetNamespace,
		exporter: root.attrs.exporter,
		exporterVersion: root.attrs.exporterVersion,
		processes: [],
		messages: rootDefs("message"),
		signals: rootDefs("signal"),
		errors: rootDefs("error", "errorCode"),
		escalations: rootDefs("escalation", "escalationCode"),
		diagram: readDiagram(root),
	};

	const refs: RootRefs = {
		messages: definitions.messages,
		signals: definitions.signals,
		errors: definitions.errors,
		escalations: definitions.escalations,
		eventDefinitions: new Map(),
	};
	for (const child of root.children) {
		if (child.name.endsWith("EventDefinition") && child.attrs.id) {
			refs.eventDefinitions.set(child.attrs.id, child);
		}
	}

	const collaborationEl = firstChild(root, "collaboration");
	if (collaborationEl) {
		definitions.collaboration = readCollaboration(collaborationEl);
	}
	const participantByProcess = new Map<string, BpmnParticipant>();
	for (const participant of definitions.collaboration?.participants ?? []) {
		if (participant.processRef) {
			participantByProcess.set(participant.processRef, participant);
		}
	}

	for (const processEl of childrenNamed(root, "process")) {
		const container = readContainer(processEl, refs);
		const id = processEl.attrs.id ?? `process_${definitions.processes.length}`;
		const participant = participantByProcess.get(id);
		definitions.processes.push({
			...container,
			id,
			name: processEl.attrs.name ?? participant?.name,
			isExecutable: processEl.attrs.isExecutable === "true",
			documentation: readDocumentation(processEl),
			participantName: participant?.name,
			vendorAttrs: readVendorAttrs(processEl),
			extensions: readExtensions(processEl),
		});
	}

	return definitions;
}

interface RootRefs {
	messages: Map<string, BpmnRootDefinition>;
	signals: Map<string, BpmnRootDefinition>;
	errors: Map<string, BpmnRootDefinition>;
	escalations: Map<string, BpmnRootDefinition>;
	/** Root-level event definitions addressable through `<eventDefinitionRef>`. */
	eventDefinitions: Map<string, XmlElement>;
}

function readCollaboration(el: XmlElement): BpmnCollaboration {
	const participants: BpmnParticipant[] = childrenNamed(el, "participant")
		.filter((p) => p.attrs.id)
		.map((p) => ({
			id: p.attrs.id,
			name: p.attrs.name,
			processRef: p.attrs.processRef,
		}));
	const messageFlows: BpmnMessageFlow[] = childrenNamed(el, "messageFlow")
		.filter((f) => f.attrs.id && f.attrs.sourceRef && f.attrs.targetRef)
		.map((f) => ({
			id: f.attrs.id,
			name: f.attrs.name,
			sourceRef: f.attrs.sourceRef,
			targetRef: f.attrs.targetRef,
			messageRef: f.attrs.messageRef,
		}));
	return {
		id: el.attrs.id ?? "collaboration",
		name: el.attrs.name,
		participants,
		messageFlows,
		annotations: readAnnotations(el),
		associations: readAssociations(el),
		groups: readGroups(el),
	};
}

function readContainer(el: XmlElement, refs: RootRefs): BpmnContainer {
	const flowNodes: BpmnFlowNode[] = [];
	const sequenceFlows: BpmnSequenceFlow[] = [];
	const dataObjects: BpmnDataObject[] = [];

	for (const child of el.children) {
		if (FLOW_NODE_KINDS.has(child.name)) {
			const node = readFlowNode(child, child.name as BpmnFlowNodeKind, refs);
			if (node) flowNodes.push(node);
			continue;
		}
		switch (child.name) {
			case "sequenceFlow": {
				const flow = readSequenceFlow(child);
				if (flow) sequenceFlows.push(flow);
				break;
			}
			case "dataObject":
			case "dataObjectReference":
			case "dataStoreReference": {
				if (!child.attrs.id) break;
				dataObjects.push({
					id: child.attrs.id,
					kind: child.name,
					name: child.attrs.name,
					ref: child.attrs.dataObjectRef ?? child.attrs.dataStoreRef,
					state:
						textOf(firstChild(child, "dataState")) ??
						firstChild(child, "dataState")?.attrs.name,
					isCollection: child.attrs.isCollection === "true",
					documentation: readDocumentation(child),
				});
				break;
			}
			default:
				break;
		}
	}

	// `<incoming>`/`<outgoing>` are optional in the schema; derive them from the
	// flows so every node knows its edges regardless of the exporting tool.
	const byId = new Map(flowNodes.map((node) => [node.id, node]));
	const seenEdges = new Map(
		flowNodes.map((node) => [
			node.id,
			{
				incoming: new Set(node.incoming),
				outgoing: new Set(node.outgoing),
			},
		]),
	);
	for (const flow of sequenceFlows) {
		const source = byId.get(flow.sourceRef);
		const target = byId.get(flow.targetRef);
		if (source && !seenEdges.get(source.id)?.outgoing.has(flow.id)) {
			seenEdges.get(source.id)?.outgoing.add(flow.id);
			source.outgoing.push(flow.id);
		}
		if (target && !seenEdges.get(target.id)?.incoming.has(flow.id)) {
			seenEdges.get(target.id)?.incoming.add(flow.id);
			target.incoming.push(flow.id);
		}
	}

	return {
		flowNodes,
		sequenceFlows,
		dataObjects,
		annotations: readAnnotations(el),
		associations: readAssociations(el),
		groups: readGroups(el),
		lanes: readLaneSets(el),
	};
}

function readAnnotations(el: XmlElement): BpmnTextAnnotation[] {
	return childrenNamed(el, "textAnnotation")
		.filter((a) => a.attrs.id)
		.map((a) => ({ id: a.attrs.id, text: textOf(firstChild(a, "text")) }));
}

function readAssociations(el: XmlElement): BpmnAssociation[] {
	return childrenNamed(el, "association")
		.filter((a) => a.attrs.id && a.attrs.sourceRef && a.attrs.targetRef)
		.map((a) => ({
			id: a.attrs.id,
			sourceRef: a.attrs.sourceRef,
			targetRef: a.attrs.targetRef,
			direction: a.attrs.associationDirection,
		}));
}

function readGroups(el: XmlElement): BpmnGroup[] {
	return childrenNamed(el, "group")
		.filter((g) => g.attrs.id)
		.map((g) => ({
			id: g.attrs.id,
			categoryValueRef: g.attrs.categoryValueRef,
			name: g.attrs.name,
		}));
}

function readLaneSets(el: XmlElement): BpmnLane[] {
	const lanes: BpmnLane[] = [];
	for (const laneSet of el.children) {
		if (laneSet.name !== "laneSet" && laneSet.name !== "childLaneSet") continue;
		for (const lane of childrenNamed(laneSet, "lane")) {
			if (!lane.attrs.id) continue;
			lanes.push({
				id: lane.attrs.id,
				name: lane.attrs.name,
				flowNodeRefs: childrenNamed(lane, "flowNodeRef")
					.map((ref) => textOf(ref))
					.filter((ref): ref is string => Boolean(ref)),
				children: readLaneSets(lane),
			});
		}
	}
	return lanes;
}

function readSequenceFlow(el: XmlElement): BpmnSequenceFlow | undefined {
	const { id, sourceRef, targetRef } = el.attrs;
	if (!id || !sourceRef || !targetRef) return undefined;
	const conditionEl = firstChild(el, "conditionExpression");
	return {
		id,
		name: el.attrs.name,
		sourceRef,
		targetRef,
		condition: textOf(conditionEl),
		conditionLanguage: conditionEl?.attrs.language,
		documentation: readDocumentation(el),
	};
}

function readDocumentation(el: XmlElement): string | undefined {
	const parts = childrenNamed(el, "documentation")
		.map((d) => textOf(d))
		.filter((d): d is string => Boolean(d));
	return parts.length > 0 ? parts.join("\n") : undefined;
}

function readVendorAttrs(el: XmlElement): Record<string, string> {
	const out: Record<string, string> = {};
	for (const [key, value] of Object.entries(el.attrs)) {
		if (key.includes(":") && !key.startsWith("xmlns")) out[key] = value;
	}
	return out;
}

function readExtensions(el: XmlElement): BpmnExtension[] {
	const holder = firstChild(el, "extensionElements");
	if (!holder) return [];
	return holder.children.map((child) => ({
		prefix: child.prefix,
		name: child.name,
		attrs: child.attrs,
		text: textOf(child),
		element: child,
	}));
}

function readRefTexts(el: XmlElement, name: string): string[] {
	return childrenNamed(el, name)
		.map((child) => textOf(child))
		.filter((text): text is string => Boolean(text));
}

function readEventDefinitions(
	el: XmlElement,
	refs: RootRefs,
): BpmnEventDefinition[] {
	const defs: BpmnEventDefinition[] = [];
	const elements: XmlElement[] = [];
	for (const child of el.children) {
		if (child.name.endsWith("EventDefinition")) elements.push(child);
		if (child.name === "eventDefinitionRef") {
			const ref = textOf(child);
			const target = ref ? refs.eventDefinitions.get(ref) : undefined;
			if (target) elements.push(target);
		}
	}
	for (const def of elements) {
		const kind = def.name.replace(/EventDefinition$/, "");
		switch (kind) {
			case "message": {
				const ref = def.attrs.messageRef;
				defs.push({
					kind,
					ref,
					refName: ref ? refs.messages.get(ref)?.name : undefined,
				});
				break;
			}
			case "signal": {
				const ref = def.attrs.signalRef;
				defs.push({
					kind,
					ref,
					refName: ref ? refs.signals.get(ref)?.name : undefined,
				});
				break;
			}
			case "error": {
				const ref = def.attrs.errorRef;
				const target = ref ? refs.errors.get(ref) : undefined;
				defs.push({ kind, ref, refName: target?.name, code: target?.code });
				break;
			}
			case "escalation": {
				const ref = def.attrs.escalationRef;
				const target = ref ? refs.escalations.get(ref) : undefined;
				defs.push({ kind, ref, refName: target?.name, code: target?.code });
				break;
			}
			case "timer":
				defs.push({
					kind,
					timer: {
						timeDate: textOf(firstChild(def, "timeDate")),
						timeCycle: textOf(firstChild(def, "timeCycle")),
						timeDuration: textOf(firstChild(def, "timeDuration")),
					},
				});
				break;
			case "conditional":
				defs.push({ kind, condition: textOf(firstChild(def, "condition")) });
				break;
			case "link":
				defs.push({ kind, linkName: def.attrs.name });
				break;
			case "compensate":
				defs.push({ kind, ref: def.attrs.activityRef });
				break;
			case "cancel":
			case "terminate":
				defs.push({ kind });
				break;
			default:
				break;
		}
	}
	return defs;
}

function readLoop(
	el: XmlElement,
	extensions: BpmnExtension[],
): BpmnLoopCharacteristics | undefined {
	const multi = firstChild(el, "multiInstanceLoopCharacteristics");
	if (multi) {
		const zeebe = extensions.find(
			(ext) => ext.prefix === "zeebe" && ext.name === "loopCharacteristics",
		);
		const multiExt = readExtensions(multi).find(
			(ext) => ext.prefix === "zeebe" && ext.name === "loopCharacteristics",
		);
		const loopExt = multiExt ?? zeebe;
		return {
			kind: "multiInstance",
			isSequential: multi.attrs.isSequential === "true",
			cardinality: textOf(firstChild(multi, "loopCardinality")),
			completionCondition: textOf(firstChild(multi, "completionCondition")),
			inputCollection:
				loopExt?.attrs.inputCollection ??
				attr(multi, "collection", { anyPrefix: true }) ??
				textOf(firstChild(multi, "loopDataInputRef")),
			inputElement:
				loopExt?.attrs.inputElement ??
				attr(multi, "elementVariable", { anyPrefix: true }) ??
				firstChild(multi, "inputDataItem")?.attrs.name,
			outputCollection:
				loopExt?.attrs.outputCollection ??
				textOf(firstChild(multi, "loopDataOutputRef")),
			outputElement:
				loopExt?.attrs.outputElement ??
				firstChild(multi, "outputDataItem")?.attrs.name,
		};
	}
	const standard = firstChild(el, "standardLoopCharacteristics");
	if (standard) {
		return {
			kind: "standard",
			isSequential: true,
			loopCondition: textOf(firstChild(standard, "loopCondition")),
			testBefore: standard.attrs.testBefore === "true",
			loopMaximum: standard.attrs.loopMaximum,
		};
	}
	return undefined;
}

/**
 * A Camunda 7 input/output parameter body, which may be plain text, a map, a
 * list or an inline script rather than a scalar. Structured bodies are
 * rendered as JSON so the value reaches the node comment instead of vanishing.
 */
function camundaParameterValue(param: XmlElement): string {
	const text = textOf(param);
	if (text) return text;
	const map = firstChild(param, "map");
	if (map) {
		return JSON.stringify(
			Object.fromEntries(
				childrenNamed(map, "entry").map((entry) => [
					entry.attrs.key ?? "",
					entry.text.trim(),
				]),
			),
		);
	}
	const list = firstChild(param, "list");
	if (list) {
		return JSON.stringify(
			childrenNamed(list, "value").map((value) => value.text.trim()),
		);
	}
	const script = firstChild(param, "script");
	if (script) {
		const format = script.attrs.scriptFormat;
		return `${format ? `${format}: ` : ""}${script.text.trim()}`;
	}
	return "";
}

function readIoMapping(extensions: BpmnExtension[]): BpmnIoMapping | undefined {
	const pairs = (el: XmlElement, name: string) =>
		childrenNamed(el, name)
			.filter(
				(p) => p.attrs.source !== undefined || p.attrs.target !== undefined,
			)
			.map((p) => ({
				source: p.attrs.source ?? "",
				target: p.attrs.target ?? "",
			}));

	const zeebe = extensions.find(
		(ext) => ext.prefix === "zeebe" && ext.name === "ioMapping",
	);
	if (zeebe) {
		return {
			inputs: pairs(zeebe.element, "input"),
			outputs: pairs(zeebe.element, "output"),
		};
	}

	// Camunda 7 connectors carry their own `inputOutput` inside `<camunda:connector>`,
	// which is where an HTTP connector's url and method live.
	const camunda =
		extensions.find(
			(ext) => ext.prefix === "camunda" && ext.name === "inputOutput",
		)?.element ??
		(() => {
			const connector = extensions.find(
				(ext) => ext.prefix === "camunda" && ext.name === "connector",
			);
			return connector
				? firstChild(connector.element, "inputOutput")
				: undefined;
		})();
	if (camunda) {
		const param = (p: XmlElement) => ({
			source: camundaParameterValue(p),
			target: p.attrs.name ?? "",
		});
		return {
			inputs: childrenNamed(camunda, "inputParameter")
				.map(param)
				.filter((io) => io.target || io.source),
			outputs: childrenNamed(camunda, "outputParameter")
				.map(param)
				.filter((io) => io.target || io.source),
		};
	}
	return undefined;
}

function readScript(
	el: XmlElement,
	extensions: BpmnExtension[],
): BpmnFlowNode["script"] {
	const inline = firstChild(el, "script");
	const zeebeScript = extensions.find(
		(ext) => ext.prefix === "zeebe" && ext.name === "script",
	);
	const camundaScript = extensions.find(
		(ext) => ext.prefix === "camunda" && ext.name === "script",
	);
	const body =
		textOf(inline) ?? zeebeScript?.attrs.expression ?? camundaScript?.text;
	const format =
		el.attrs.scriptFormat ??
		camundaScript?.attrs.scriptFormat ??
		(zeebeScript ? "feel" : undefined);
	const resultVariable =
		attr(el, "resultVariable", { anyPrefix: true }) ??
		zeebeScript?.attrs.resultVariable;
	if (!body && !format && !resultVariable) return undefined;
	return { format, body, resultVariable };
}

function readCalledElement(
	el: XmlElement,
	extensions: BpmnExtension[],
): string | undefined {
	const zeebe = extensions.find(
		(ext) => ext.prefix === "zeebe" && ext.name === "calledElement",
	);
	return el.attrs.calledElement ?? zeebe?.attrs.processId;
}

function readFlowNode(
	el: XmlElement,
	kind: BpmnFlowNodeKind,
	refs: RootRefs,
): BpmnFlowNode | undefined {
	const id = el.attrs.id;
	if (!id) return undefined;
	const extensions = readExtensions(el);

	const dataInputs: string[] = [];
	for (const assoc of childrenNamed(el, "dataInputAssociation")) {
		dataInputs.push(...readRefTexts(assoc, "sourceRef"));
	}
	const dataOutputs: string[] = [];
	for (const assoc of childrenNamed(el, "dataOutputAssociation")) {
		dataOutputs.push(...readRefTexts(assoc, "targetRef"));
	}

	const isBoundary = kind === "boundaryEvent";
	const interrupting = isBoundary
		? el.attrs.cancelActivity !== "false"
		: el.attrs.isInterrupting !== "false";

	const node: BpmnFlowNode = {
		id,
		kind,
		name: el.attrs.name,
		documentation: readDocumentation(el),
		incoming: readRefTexts(el, "incoming"),
		outgoing: readRefTexts(el, "outgoing"),
		eventDefinitions: readEventDefinitions(el, refs),
		interrupting,
		attachedToRef: el.attrs.attachedToRef,
		defaultFlow: el.attrs.default,
		eventGatewayType: el.attrs.eventGatewayType,
		instantiate: el.attrs.instantiate === "true" ? true : undefined,
		triggeredByEvent: el.attrs.triggeredByEvent === "true" ? true : undefined,
		calledElement:
			kind === "callActivity" ? readCalledElement(el, extensions) : undefined,
		script: kind === "scriptTask" ? readScript(el, extensions) : undefined,
		loop: readLoop(el, extensions),
		ioMapping: readIoMapping(extensions),
		dataInputs,
		dataOutputs,
		isForCompensation: el.attrs.isForCompensation === "true",
		vendorAttrs: readVendorAttrs(el),
		extensions,
	};

	if (BPMN_SUBPROCESS_KINDS.has(kind)) {
		node.body = readContainer(el, refs);
	}
	return node;
}

function readNumber(value: string | undefined): number {
	const parsed = Number.parseFloat(value ?? "");
	return Number.isFinite(parsed) ? parsed : 0;
}

function readDiagram(root: XmlElement): BpmnDiagram {
	const shapes = new Map<string, BpmnShape>();
	const edges = new Map<string, Array<{ x: number; y: number }>>();
	for (const diagram of childrenNamed(root, "BPMNDiagram")) {
		for (const plane of childrenNamed(diagram, "BPMNPlane")) {
			for (const shape of childrenNamed(plane, "BPMNShape")) {
				const elementId = shape.attrs.bpmnElement;
				const bounds = firstChild(shape, "Bounds");
				if (!elementId || !bounds || shapes.has(elementId)) continue;
				shapes.set(elementId, {
					x: readNumber(bounds.attrs.x),
					y: readNumber(bounds.attrs.y),
					width: readNumber(bounds.attrs.width),
					height: readNumber(bounds.attrs.height),
					isExpanded:
						shape.attrs.isExpanded === undefined
							? undefined
							: shape.attrs.isExpanded === "true",
					isHorizontal:
						shape.attrs.isHorizontal === undefined
							? undefined
							: shape.attrs.isHorizontal === "true",
				});
			}
			for (const edge of childrenNamed(plane, "BPMNEdge")) {
				const elementId = edge.attrs.bpmnElement;
				if (!elementId || edges.has(elementId)) continue;
				edges.set(
					elementId,
					childrenNamed(edge, "waypoint").map((wp) => ({
						x: readNumber(wp.attrs.x),
						y: readNumber(wp.attrs.y),
					})),
				);
			}
		}
	}
	return { shapes, edges };
}

/** Every flow node in a container and, recursively, in its sub-processes. */
export function* walkFlowNodes(
	container: BpmnContainer,
): Generator<BpmnFlowNode> {
	for (const node of container.flowNodes) {
		yield node;
		if (node.body) yield* walkFlowNodes(node.body);
	}
}

/** Every sequence flow in a container and, recursively, in its sub-processes. */
export function* walkSequenceFlows(
	container: BpmnContainer,
): Generator<BpmnSequenceFlow> {
	yield* container.sequenceFlows;
	for (const node of container.flowNodes) {
		if (node.body) yield* walkSequenceFlows(node.body);
	}
}

export function isEventKind(kind: BpmnFlowNodeKind): kind is BpmnEventKind {
	return (BPMN_EVENT_KINDS as readonly string[]).includes(kind);
}

export function isGatewayKind(kind: BpmnFlowNodeKind): kind is BpmnGatewayKind {
	return (BPMN_GATEWAY_KINDS as readonly string[]).includes(kind);
}

export function isActivityKind(
	kind: BpmnFlowNodeKind,
): kind is BpmnActivityKind {
	return (BPMN_ACTIVITY_KINDS as readonly string[]).includes(kind);
}

/** A vendor extension by prefix + local name, e.g. `("zeebe", "taskDefinition")`. */
export function findExtension(
	node: { extensions: BpmnExtension[] },
	prefix: string,
	name: string,
): BpmnExtension | undefined {
	return node.extensions.find(
		(ext) => ext.prefix === prefix && ext.name === name,
	);
}

/**
 * Parses an ISO 8601 duration (`PT5M`, `P1DT2H`, `PT0.5S`) into seconds.
 * Months and years have no fixed length; they are approximated at 30 and 365 days.
 */
export function iso8601DurationToSeconds(value: string): number | undefined {
	const match =
		/^P(?:(\d+(?:\.\d+)?)Y)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)W)?(?:(\d+(?:\.\d+)?)D)?(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$/i.exec(
			value.trim(),
		);
	if (!match) return undefined;
	const [, years, months, weeks, days, hours, minutes, seconds] = match;
	// Every component is optional in the pattern, so a bare `P` or `PT` matches
	// with nothing in it; that is a malformed duration, not zero.
	if (match.slice(1).every((part) => part === undefined)) return undefined;
	const num = (part: string | undefined) =>
		part ? Number.parseFloat(part) : 0;
	const total =
		num(years) * 365 * 86400 +
		num(months) * 30 * 86400 +
		num(weeks) * 7 * 86400 +
		num(days) * 86400 +
		num(hours) * 3600 +
		num(minutes) * 60 +
		num(seconds);
	return Number.isFinite(total) ? total : undefined;
}

const DURATION_UNITS: Array<[RegExp, number]> = [
	[/^(?:ms|millisecs?|milliseconds?)$/i, 0.001],
	[/^(?:s|secs?|seconds?)$/i, 1],
	[/^(?:m|mins?|minutes?)$/i, 60],
	[/^(?:h|hrs?|hours?)$/i, 3600],
	[/^(?:d|days?)$/i, 86400],
	[/^(?:w|wks?|weeks?)$/i, 604800],
];

/**
 * A duration written as a label rather than a value — "60 minutes", "after 2h".
 * Descriptive BPMN routinely leaves `timerEventDefinition` empty and puts the
 * wait in the event's name. Deliberately strict: a label that is not *only* a
 * duration returns `undefined` rather than a guess.
 */
export function labelDurationToSeconds(label: string): number | undefined {
	const match =
		/^(?:after|in|wait(?:\s+for)?)?\s*(\d+(?:[.,]\d+)?)\s*([a-z]+)\.?$/i.exec(
			label.trim(),
		);
	if (!match) return undefined;
	const amount = Number.parseFloat(match[1].replace(",", "."));
	if (!Number.isFinite(amount)) return undefined;
	const unit = DURATION_UNITS.find(([pattern]) => pattern.test(match[2]));
	return unit ? amount * unit[1] : undefined;
}

/**
 * The per-repetition duration of an ISO 8601 repeating interval such as
 * `R3/PT1H` or `R/P1D`; `undefined` for cron strings and dated cycles.
 */
export function iso8601CycleDurationToSeconds(
	value: string,
): number | undefined {
	const match = /^R\d*\/(P.+)$/i.exec(value.trim());
	return match ? iso8601DurationToSeconds(match[1]) : undefined;
}
