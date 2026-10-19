export type BoardNodeId =
	| "email"
	| "extract"
	| "enrich"
	| "approve"
	| "gov"
	| "data"
	| "ontology"
	| "notes"
	| "pilot"
	| "calendar";

/** Header tint of the product painter: event = primary, pure = tertiary, function = violet, standard = none. */
export type BoardNodeKind = "event" | "standard" | "pure" | "function";
export type BoardPinKind = "execution" | "data" | "reference";
export type BoardDirection = "input" | "output";
export type BoardDataType = "Struct" | "String";
export type BoardValueType = "Execution" | "FunctionRef" | BoardDataType;
export type BoardScenarioId = "approval" | "insight" | "flowpilot";

export const BOARD_STRING_KEYS = [
	"v5.apps.connected.email.title",
	"v5.apps.node.extract",
	"v5.apps.node.enrich",
	"v5.apps.node.approve",
	"v5.apps.node.governance",
	"v5.platform.f.data.title",
	"v5.apps.node.ontology",
	"v5.apps.connected.notes.title",
	"v5.apps.node.flowpilot",
	"v5.apps.connected.calendar.title",
	"v5.apps.mini.agent.mail",
	"v5.apps.mini.agent.data",
	"v5.apps.board.pin.message",
	"v5.apps.board.pin.request",
	"v5.apps.board.pin.companyData",
	"v5.apps.board.pin.policy",
	"v5.apps.board.pin.records",
	"v5.apps.board.pin.meetingNotes",
	"v5.apps.board.pin.briefing",
	"v5.apps.board.pin.agenda",
] as const;

export type BoardStringKey = (typeof BOARD_STRING_KEYS)[number];
export type BoardStrings = Readonly<Record<BoardStringKey, string>>;

export interface BoardPin {
	readonly id: string;
	readonly direction: BoardDirection;
	readonly kind: BoardPinKind;
	readonly type: BoardValueType;
	/** Empty for execution and reference pins, which the product never labels. */
	readonly label: string;
}

export interface BoardNode {
	readonly id: BoardNodeId;
	readonly kind: BoardNodeKind;
	readonly column: number;
	readonly row: number;
	/** Pin rows to shift the node down inside its row band, so a wire into a lower pin runs straight. */
	readonly offset: number;
	readonly title: string;
	readonly pins: readonly BoardPin[];
}

export interface BoardEndpoint {
	readonly node: BoardNodeId;
	readonly pin: string;
}

export interface BoardEdge {
	readonly id: string;
	readonly kind: BoardPinKind;
	/** The source pin's type, which colours data wires. */
	readonly type: BoardValueType;
	readonly source: BoardEndpoint;
	readonly target: BoardEndpoint;
}

/** An app window connected to a node's top-centre reference pin. `app` indexes `DescentData.apps`. */
export interface BoardThread {
	readonly id: string;
	readonly app: number;
	readonly node: BoardNodeId;
	readonly pin: string;
}

export interface BoardThreadHop {
	readonly id: string;
	/** True when work travels up from the board into the app window. */
	readonly toApp: boolean;
}

/** Hops that travel together. `stage` indexes the scenario's route pills (source, processing, arrival). */
export interface BoardBeat {
	readonly stage: number;
	readonly edges: readonly string[];
	readonly threads: readonly BoardThreadHop[];
}

export interface BoardScenario {
	readonly id: BoardScenarioId;
	readonly beats: readonly BoardBeat[];
}

export interface BoardSpec {
	readonly nodes: readonly BoardNode[];
	readonly edges: readonly BoardEdge[];
	readonly threads: readonly BoardThread[];
	readonly scenarios: readonly BoardScenario[];
}

interface PinDef {
	id: string;
	direction: BoardDirection;
	type: BoardValueType;
	label?: BoardStringKey;
}

interface NodeDef {
	id: BoardNodeId;
	kind: BoardNodeKind;
	column: number;
	row: number;
	offset?: number;
	title: BoardStringKey;
	pins: readonly PinDef[];
}

export const REFERENCE_PIN = "ref";

const ref: PinDef = {
	id: REFERENCE_PIN,
	direction: "input",
	type: "FunctionRef",
};
const execIn: PinDef = { id: "exec_in", direction: "input", type: "Execution" };
const execOut: PinDef = {
	id: "exec_out",
	direction: "output",
	type: "Execution",
};
const input = (
	id: string,
	type: BoardDataType,
	label: BoardStringKey,
): PinDef => ({ id, direction: "input", type, label });
const output = (
	id: string,
	type: BoardDataType,
	label: BoardStringKey,
): PinDef => ({ id, direction: "output", type, label });

// Grid: row 0 holds the data and governance sources the apps query, row 1 FlowPilot and
// its schedule, row 2 the approval workflow. Every node with a thread keeps the cells
// above it empty, so threads drop onto their reference pins without crossing a node.
const NODES: readonly NodeDef[] = [
	{
		id: "data",
		kind: "pure",
		column: 0,
		row: 0,
		title: "v5.platform.f.data.title",
		pins: [ref, output("records", "Struct", "v5.apps.board.pin.records")],
	},
	{
		id: "ontology",
		kind: "pure",
		column: 2,
		row: 0,
		title: "v5.apps.node.ontology",
		pins: [
			ref,
			input("records", "Struct", "v5.apps.board.pin.records"),
			output("company_data", "Struct", "v5.apps.board.pin.companyData"),
		],
	},
	{
		id: "gov",
		kind: "pure",
		column: 3,
		row: 0,
		title: "v5.apps.node.governance",
		pins: [ref, output("policy", "Struct", "v5.apps.board.pin.policy")],
	},
	{
		id: "notes",
		kind: "pure",
		column: 0,
		row: 1,
		offset: 1,
		title: "v5.apps.connected.notes.title",
		pins: [output("meeting_notes", "String", "v5.apps.board.pin.meetingNotes")],
	},
	{
		// Invoked through its reference square like a product event, so it has no exec input.
		id: "pilot",
		kind: "function",
		column: 1,
		row: 1,
		title: "v5.apps.node.flowpilot",
		pins: [
			ref,
			input("data", "Struct", "v5.apps.mini.agent.data"),
			input("notes", "String", "v5.apps.connected.notes.title"),
			input("mail", "Struct", "v5.apps.mini.agent.mail"),
			execOut,
			output("briefing", "String", "v5.apps.board.pin.briefing"),
		],
	},
	{
		id: "calendar",
		kind: "standard",
		column: 2,
		row: 1,
		title: "v5.apps.connected.calendar.title",
		pins: [
			execIn,
			input("agenda", "String", "v5.apps.board.pin.agenda"),
			execOut,
		],
	},
	{
		id: "email",
		kind: "event",
		column: 0,
		row: 2,
		title: "v5.apps.connected.email.title",
		pins: [execOut, output("message", "Struct", "v5.apps.board.pin.message")],
	},
	{
		id: "extract",
		kind: "standard",
		column: 1,
		row: 2,
		title: "v5.apps.node.extract",
		pins: [
			execIn,
			input("message", "Struct", "v5.apps.board.pin.message"),
			execOut,
			output("request", "Struct", "v5.apps.board.pin.request"),
		],
	},
	{
		id: "enrich",
		kind: "standard",
		column: 3,
		row: 2,
		title: "v5.apps.node.enrich",
		pins: [
			execIn,
			input("request", "Struct", "v5.apps.board.pin.request"),
			input("company_data", "Struct", "v5.apps.board.pin.companyData"),
			execOut,
			output("enriched", "Struct", "v5.apps.board.pin.request"),
		],
	},
	{
		id: "approve",
		kind: "standard",
		column: 4,
		row: 2,
		title: "v5.apps.node.approve",
		pins: [
			ref,
			execIn,
			input("request", "Struct", "v5.apps.board.pin.request"),
			input("policy", "Struct", "v5.apps.board.pin.policy"),
			execOut,
		],
	},
];

const EDGES: readonly (readonly [
	id: string,
	source: BoardNodeId,
	sourcePin: string,
	target: BoardNodeId,
	targetPin: string,
])[] = [
	["email-extract-exec", "email", "exec_out", "extract", "exec_in"],
	["email-extract-message", "email", "message", "extract", "message"],
	["email-pilot-mail", "email", "message", "pilot", "mail"],
	["extract-enrich-exec", "extract", "exec_out", "enrich", "exec_in"],
	["extract-enrich-request", "extract", "request", "enrich", "request"],
	[
		"ontology-enrich-data",
		"ontology",
		"company_data",
		"enrich",
		"company_data",
	],
	["enrich-approve-exec", "enrich", "exec_out", "approve", "exec_in"],
	["enrich-approve-request", "enrich", "enriched", "approve", "request"],
	["gov-approve-policy", "gov", "policy", "approve", "policy"],
	["data-ontology-records", "data", "records", "ontology", "records"],
	["data-pilot-data", "data", "records", "pilot", "data"],
	["notes-pilot-notes", "notes", "meeting_notes", "pilot", "notes"],
	["pilot-calendar-exec", "pilot", "exec_out", "calendar", "exec_in"],
	["pilot-calendar-agenda", "pilot", "briefing", "calendar", "agenda"],
];

// App indices: 0 Operations HQ, 1 Finance Approvals, 2 Customer 360, 3 Knowledge Graph,
// 4 AI Control Center, 5 Inventory Planner, 6 FlowPilot Workspace, 7 Field Service.
const THREADS: readonly (readonly [
	id: string,
	app: number,
	node: BoardNodeId,
])[] = [
	["finance", 1, "approve"],
	["field", 7, "approve"],
	["graph", 3, "data"],
	["inventory", 5, "data"],
	["ops-ontology", 0, "ontology"],
	["customer", 2, "ontology"],
	["control", 4, "gov"],
	["pilot", 6, "pilot"],
	["ops-pilot", 0, "pilot"],
];

const beat = (
	stage: number,
	edges: readonly string[],
	threads: readonly BoardThreadHop[] = [],
): BoardBeat => ({ stage, edges, threads });

const SCENARIOS: readonly BoardScenario[] = [
	{
		id: "approval",
		beats: [
			beat(0, ["email-extract-exec", "email-extract-message"]),
			beat(1, [
				"extract-enrich-exec",
				"extract-enrich-request",
				"ontology-enrich-data",
			]),
			beat(1, [
				"enrich-approve-exec",
				"enrich-approve-request",
				"gov-approve-policy",
			]),
			beat(2, [], [{ id: "finance", toApp: true }]),
		],
	},
	{
		id: "insight",
		beats: [
			beat(0, [], [{ id: "graph", toApp: false }]),
			beat(1, ["data-ontology-records"]),
			beat(2, [], [{ id: "ops-ontology", toApp: true }]),
		],
	},
	{
		id: "flowpilot",
		beats: [
			beat(0, [], [{ id: "pilot", toApp: false }]),
			beat(1, ["data-pilot-data", "notes-pilot-notes", "email-pilot-mail"]),
			beat(
				1,
				["pilot-calendar-exec", "pilot-calendar-agenda"],
				[{ id: "ops-pilot", toApp: true }],
			),
			beat(2, [], [{ id: "pilot", toApp: true }]),
		],
	},
];

const pinKind = (type: BoardValueType): BoardPinKind =>
	type === "Execution"
		? "execution"
		: type === "FunctionRef"
			? "reference"
			: "data";

/** Resolves the board graph with translated titles and pin labels. */
export function createBoard(strings: BoardStrings): BoardSpec {
	const nodes = NODES.map(
		(node): BoardNode => ({
			id: node.id,
			kind: node.kind,
			column: node.column,
			row: node.row,
			offset: node.offset ?? 0,
			title: strings[node.title],
			pins: node.pins.map((pin) => ({
				id: pin.id,
				direction: pin.direction,
				kind: pinKind(pin.type),
				type: pin.type,
				label: pin.label ? strings[pin.label] : "",
			})),
		}),
	);
	const typeOf = (node: BoardNodeId, pin: string): BoardValueType => {
		const found = NODES.find((def) => def.id === node)?.pins.find(
			(def) => def.id === pin,
		);
		if (!found) throw new Error(`Board edge names unknown pin ${node}.${pin}`);
		return found.type;
	};
	const edges = EDGES.map(
		([id, source, sourcePin, target, targetPin]): BoardEdge => {
			const type = typeOf(source, sourcePin);
			return {
				id,
				kind: pinKind(type),
				type,
				source: { node: source, pin: sourcePin },
				target: { node: target, pin: targetPin },
			};
		},
	);
	const threads = THREADS.map(
		([id, app, node]): BoardThread => ({ id, app, node, pin: REFERENCE_PIN }),
	);
	return { nodes, edges, threads, scenarios: SCENARIOS };
}
