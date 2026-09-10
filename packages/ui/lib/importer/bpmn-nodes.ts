import { createId } from "@paralleldrive/cuid2";
import type { IBoard, INode, IPin } from "../schema";
import { IPinType, IValueType, IVariableType } from "../schema";
import {
	type CatalogIndex,
	cloneNodeFromCatalog,
	createNode,
	encodeJsonDefault,
} from "./board-builder";

/**
 * The catalog nodes the BPMN translator places, with the pin contract each
 * one has in the Rust catalog (verified against `get_node()`), so the
 * translator can run without a live catalog — in tests, or on a client whose
 * catalog query has not resolved — and still mint nodes the backend will
 * recognise by name and pin name on paste.
 *
 * Pin names here MUST match the catalog exactly: `copy_paste.rs` matches
 * pasted pins to the blueprint by `(name, pin_type)` and
 * `sync_node_with_catalog` removes unwired pins it does not know.
 */

export interface FallbackPinSpec {
	name: string;
	friendly: string;
	type: IPinType;
	data: IVariableType;
	value?: IValueType;
	default?: unknown;
}

export interface FallbackNodeSpec {
	friendly: string;
	category: string;
	start?: boolean;
	pins: FallbackPinSpec[];
}

/** An execution pin spec; also what a Function layer's boundary pins are made of. */
export const execPinSpec = (
	name: string,
	friendly: string,
	type: IPinType,
): FallbackPinSpec => ({
	name,
	friendly,
	type,
	data: IVariableType.Execution,
});
const exec = execPinSpec;
const input = (
	name: string,
	friendly: string,
	data: IVariableType,
	def?: unknown,
	value?: IValueType,
): FallbackPinSpec => ({
	name,
	friendly,
	type: IPinType.Input,
	data,
	default: def,
	value,
});
const output = (
	name: string,
	friendly: string,
	data: IVariableType,
	value?: IValueType,
): FallbackPinSpec => ({
	name,
	friendly,
	type: IPinType.Output,
	data,
	value,
});

const EXEC_IN = exec("exec_in", "Input", IPinType.Input);
const EXEC_OUT = exec("exec_out", "Output", IPinType.Output);

export const BPMN_NODE_SPECS: Record<string, FallbackNodeSpec> = {
	events_simple: {
		friendly: "Simple Event",
		category: "Events",
		start: true,
		pins: [EXEC_OUT],
	},
	events_generic: {
		friendly: "Generic Event",
		category: "Events",
		start: true,
		pins: [
			exec("exec_out", "Exec Out", IPinType.Output),
			output("payload", "Payload", IVariableType.Struct),
		],
	},
	log_info: {
		friendly: "Print Info",
		category: "Logging",
		pins: [
			EXEC_IN,
			input("message", "Message", IVariableType.Generic, ""),
			input("toast", "On Screen?", IVariableType.Boolean, false),
			EXEC_OUT,
		],
	},
	log_warning: {
		friendly: "Log Warning",
		category: "Logging",
		pins: [
			EXEC_IN,
			input("message", "Message", IVariableType.Generic, ""),
			input("toast", "On Screen?", IVariableType.Boolean, false),
			EXEC_OUT,
		],
	},
	log_error: {
		friendly: "Log Error",
		category: "Logging",
		pins: [
			EXEC_IN,
			input("message", "Message", IVariableType.Generic, ""),
			input("toast", "On Screen?", IVariableType.Boolean, false),
			EXEC_OUT,
		],
	},
	flow_assert: {
		friendly: "Assert",
		category: "Utils/Testing",
		pins: [
			EXEC_IN,
			input("condition", "Condition", IVariableType.Boolean, false),
			input("label", "Label", IVariableType.String, "assertion"),
			input("details", "Details", IVariableType.Generic, ""),
			exec("exec_out", "Pass", IPinType.Output),
		],
	},
	control_branch: {
		friendly: "Branch",
		category: "Control",
		pins: [
			EXEC_IN,
			input("condition", "Condition", IVariableType.Boolean, true),
			exec("true", "True", IPinType.Output),
			exec("false", "False", IPinType.Output),
		],
	},
	control_sequence: {
		friendly: "Sequence",
		category: "Control",
		pins: [EXEC_IN, EXEC_OUT, EXEC_OUT],
	},
	control_par_execution: {
		friendly: "Parallel Execution",
		category: "Control",
		pins: [
			EXEC_IN,
			input("thread_model", "Threads", IVariableType.String, "tasks"),
			exec("exec_out", "Parallel Task", IPinType.Output),
			exec("exec_out", "Parallel Task", IPinType.Output),
			exec("exec_done", "Done", IPinType.Output),
		],
	},
	control_switch: {
		friendly: "Switch",
		category: "Control/Flow",
		pins: [
			exec("exec_in", "In", IPinType.Input),
			input("value", "Value", IVariableType.Generic),
			input("cases", "Cases", IVariableType.String, ""),
			exec("default", "Default", IPinType.Output),
			output("matched_case", "Matched Case", IVariableType.String),
		],
	},
	control_gather: {
		friendly: "Gather",
		category: "Control/Parallel",
		pins: [EXEC_IN, EXEC_IN, exec("exec_done", "In Sync", IPinType.Output)],
	},
	control_timeout: {
		friendly: "Timeout",
		category: "Control",
		pins: [
			exec("exec_in", "Execute", IPinType.Input),
			input("timeout_ms", "Timeout (ms)", IVariableType.Float, 5000),
			exec("exec_body", "Execute", IPinType.Output),
			exec("exec_completed", "Completed", IPinType.Output),
			exec("exec_timed_out", "Timed Out", IPinType.Output),
		],
	},
	control_for_each: {
		friendly: "For Each",
		category: "Control",
		pins: [
			EXEC_IN,
			input(
				"array",
				"Array",
				IVariableType.Generic,
				undefined,
				IValueType.Array,
			),
			exec("exec_out", "For Each Element", IPinType.Output),
			output("value", "Value", IVariableType.Generic),
			output("index", "Index", IVariableType.Integer),
			exec("done", "Done", IPinType.Output),
		],
	},
	control_par_for_each: {
		friendly: "Parallel For Each",
		category: "Control",
		pins: [
			EXEC_IN,
			input(
				"array",
				"Array",
				IVariableType.Generic,
				undefined,
				IValueType.Array,
			),
			input("max_concurrent", "Max Concurrent", IVariableType.Integer, 30),
			exec("exec_out", "For Each Element", IPinType.Output),
			output("value", "Value", IVariableType.Generic),
			output("index", "Index", IVariableType.Integer),
			exec("done", "Done", IPinType.Output),
		],
	},
	control_while_loop: {
		friendly: "While Loop",
		category: "Control",
		pins: [
			EXEC_IN,
			input("condition", "Condition", IVariableType.Boolean, false),
			input("max_iter", "Max", IVariableType.Integer, 15),
			exec("exec_out", "Downstream Execution", IPinType.Output),
			output("iter", "Iter", IVariableType.Integer),
			exec("done", "Done", IPinType.Output),
		],
	},
	delay: {
		friendly: "Delay",
		category: "Control",
		pins: [
			exec("exec_in", "Execute", IPinType.Input),
			input("time", "Time (ms)", IVariableType.Float, 1000),
			exec("exec_out", "Done", IPinType.Output),
		],
	},
	variable_get: {
		friendly: "Get Variable",
		category: "Variable",
		pins: [
			input("var_ref", "Variable Reference", IVariableType.String),
			output("value_ref", "Value", IVariableType.Generic),
		],
	},
	variable_set: {
		friendly: "Set Variable",
		category: "Variable",
		pins: [
			EXEC_IN,
			input("var_ref", "Variable Reference", IVariableType.String),
			input("value_in", "Value", IVariableType.Generic),
			EXEC_OUT,
			output("value_ref", "New Value", IVariableType.Generic),
		],
	},
	control_call_function: {
		friendly: "Call Function",
		category: "Control/Functions",
		pins: [input("function_layer_id", "Function", IVariableType.String)],
	},
	http_make_request: {
		friendly: "Make Request",
		category: "Web/API/Request",
		pins: [
			input("method", "Method", IVariableType.String, "GET"),
			input("url", "URL", IVariableType.String, ""),
			output("request", "Request", IVariableType.Struct),
		],
	},
	http_fetch: {
		friendly: "Fetch",
		category: "Web/API",
		pins: [
			EXEC_IN,
			input("request", "Request", IVariableType.Struct),
			exec("exec_success", "Success", IPinType.Output),
			output("response", "Response", IVariableType.Struct),
			exec("exec_error", "Error", IPinType.Output),
		],
	},
	http_response_to_json: {
		friendly: "To JSON",
		category: "Web/API/Response",
		pins: [
			EXEC_IN,
			input("response", "Response", IVariableType.Struct),
			EXEC_OUT,
			output("struct", "Struct", IVariableType.Struct),
		],
	},
	email_smtp_connect: {
		friendly: "SMTP Connect",
		category: "Email/SMTP",
		pins: [
			EXEC_IN,
			input("host", "Host", IVariableType.String, "smtp.example.com"),
			input("port", "Port", IVariableType.Integer, 587),
			input("username", "Username", IVariableType.String, ""),
			input("password", "Password", IVariableType.String, ""),
			input("encryption", "Encryption", IVariableType.String, "StartTls"),
			EXEC_OUT,
			output("connection", "Connection", IVariableType.Struct),
		],
	},
	email_smtp_send: {
		friendly: "Send Mail",
		category: "Email/SMTP",
		pins: [
			EXEC_IN,
			input("connection", "Connection", IVariableType.Struct),
			input("from", "From", IVariableType.String, ""),
			input("to", "To", IVariableType.String, ""),
			input("subject", "Subject", IVariableType.String, "(no subject)"),
			input("body_text", "Body (text)", IVariableType.String, ""),
			EXEC_OUT,
			output("message_id", "Message ID", IVariableType.String),
		],
	},
	ai_generative_find_model: {
		friendly: "Find Model",
		category: "AI/Generative",
		pins: [
			EXEC_IN,
			input("preferences", "Preferences", IVariableType.Struct),
			EXEC_OUT,
			output("model", "Model", IVariableType.Struct),
		],
	},
	ai_generative_invoke_simple: {
		friendly: "Invoke Simple",
		category: "AI/Generative",
		pins: [
			EXEC_IN,
			input("model", "Model", IVariableType.Struct),
			input("system_prompt", "System Prompt", IVariableType.String, ""),
			input("prompt", "Prompt", IVariableType.String, ""),
			input("stream", "Stream", IVariableType.Boolean, true),
			exec("on_stream", "On Stream", IPinType.Output),
			output("token", "Token", IVariableType.String),
			exec("done", "Done", IPinType.Output),
			output("result", "Result", IVariableType.String),
		],
	},
};

export interface PlaceNodeOptions {
	x: number;
	y: number;
	layer: string | null;
	friendlyName?: string;
	comment?: string;
	start?: boolean;
}

/**
 * Whether this node can be placed. A live catalog is authoritative — a
 * deployment without the web or mail nodes must get placeholders, not nodes
 * it cannot run — and the fallback table only stands in when there is none.
 */
export function catalogHas(
	catalog: CatalogIndex | undefined,
	name: string,
): boolean {
	return catalog ? catalog.has(name) : Boolean(BPMN_NODE_SPECS[name]);
}

/**
 * Places a catalog node on the board: cloned from the live catalog when there
 * is one, otherwise minted from the fallback spec so the pin contract is
 * still right. Returns `undefined` when the node cannot be placed.
 */
export function placeCatalogNode(
	board: IBoard,
	catalog: CatalogIndex | undefined,
	name: string,
	opts: PlaceNodeOptions,
): INode | undefined {
	const spec = BPMN_NODE_SPECS[name];
	const friendlyName = opts.friendlyName ?? spec?.friendly ?? name;
	let node = catalog
		? cloneNodeFromCatalog(catalog, name, {
				friendlyName,
				x: opts.x,
				y: opts.y,
				layer: opts.layer ?? undefined,
				comment: opts.comment,
				start: opts.start,
			})
		: undefined;
	if (!node) {
		if (catalog || !spec) return undefined;
		node = createNode({
			name,
			friendlyName,
			description: "",
			category: spec.category,
			x: opts.x,
			y: opts.y,
			layer: opts.layer ?? undefined,
			comment: opts.comment,
			start: opts.start ?? spec.start,
		});
		for (const pin of spec.pins) mintPin(node, pin);
	}
	node.layer = opts.layer;
	board.nodes[node.id] = node;
	return node;
}

/**
 * Adds a pin with the next free index for its direction. Takes anything with a
 * pin map, so a layer's boundary pins are minted the same way a node's are.
 */
export function mintPin(
	node: { pins: Record<string, IPin> },
	spec: FallbackPinSpec,
): IPin {
	const index =
		Object.values(node.pins).filter((p) => p.pin_type === spec.type).length + 1;
	const pin: IPin = {
		id: createId(),
		name: spec.name,
		friendly_name: spec.friendly,
		description: "",
		pin_type: spec.type,
		data_type: spec.data,
		value_type: spec.value ?? IValueType.Normal,
		default_value:
			spec.default !== undefined ? encodeJsonDefault(spec.default) : null,
		connected_to: [],
		depends_on: [],
		index,
		options: null,
		schema: null,
	};
	node.pins[pin.id] = pin;
	return pin;
}

export function pinsNamed(node: INode, name: string, type: IPinType): IPin[] {
	return Object.values(node.pins)
		.filter((p) => p.name === name && p.pin_type === type)
		.sort((a, b) => a.index - b.index);
}

export function inputPin(node: INode, name: string): IPin | undefined {
	return pinsNamed(node, name, IPinType.Input)[0];
}

export function outputPin(node: INode, name: string): IPin | undefined {
	return pinsNamed(node, name, IPinType.Output)[0];
}

/**
 * Same-named pins are the catalog's "add another" idiom (`exec_out` on
 * Sequence and Parallel Execution, `exec_in` on Gather). Grows the set to
 * `count` by cloning the first one, and returns all of them in index order.
 */
export function ensurePins(
	node: INode,
	name: string,
	type: IPinType,
	count: number,
): IPin[] {
	const existing = pinsNamed(node, name, type);
	const template = existing[0];
	if (!template) return existing;
	for (let i = existing.length; i < count; i += 1) {
		mintPin(node, {
			name,
			friendly: template.friendly_name,
			type,
			data: template.data_type as IVariableType,
			value: template.value_type as IValueType,
		});
	}
	return pinsNamed(node, name, type);
}

const CASE_PREFIX = "case_";

/**
 * The output pin names `control_switch` derives from its cases. Mirrors
 * `case_pin_names` in the catalog: a pin the importer mints under a different
 * name would be dropped, and its wire with it, the first time `on_update` runs.
 */
export function switchCasePinNames(cases: readonly string[]): string[] {
	const names: string[] = [];
	for (const value of cases) {
		const sanitized = [...value]
			.map((character) =>
				/[a-zA-Z0-9]/.test(character) ? character.toLowerCase() : "_",
			)
			.join("");
		const base = `${CASE_PREFIX}${sanitized.replace(/^_+|_+$/g, "")}`;
		let name = base;
		for (let suffix = 2; names.includes(name); suffix += 1) {
			name = `${base}_${suffix}`;
		}
		names.push(name);
	}
	return names;
}

export function setDefault(node: INode, pinName: string, value: unknown): void {
	const pin = inputPin(node, pinName);
	if (pin) pin.default_value = encodeJsonDefault(value);
}
