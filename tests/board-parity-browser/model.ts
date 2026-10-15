import {
	type IBoard,
	ICommentType,
	IExecutionMode,
	IExecutionStage,
	ILayerType,
	ILogLevel,
} from "../../packages/ui/lib/schema/flow/board";
import {
	ICommandType,
	type IGenericCommand,
} from "../../packages/ui/lib/schema/flow/board/commands/generic-command";
import { applyCommands, makeBoard } from "../board-canvas-browser/model";
const time = { secs_since_epoch: 0, nanos_since_epoch: 0 };
export const icon = `data:image/svg+xml,${encodeURIComponent(
	'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" rx="12" fill="#22c55e"/><path d="M12 32h40M32 12v40" stroke="white" stroke-width="8"/></svg>',
)}`;
export function makeParityBoard(count = 12): IBoard {
	const nodes = makeBoard(Math.max(10, count - 2));
	Object.assign(nodes["string-a"], {
		friendly_name: "Validation error",
		error: "Required schema is missing",
		comment: "Verify the incoming payload before running.",
	});
	Object.assign(nodes["string-b"], { friendly_name: "Custom icon", icon });
	Object.assign(nodes["string-c"], { friendly_name: "Dropdown input" });
	nodes["string-c"].pins["string-c.in"].options = {
		valid_values: ["first", "second", "third"],
	};
	nodes["string-c"].pins["string-c.in"].default_value = Array.from(
		new TextEncoder().encode(JSON.stringify("first")),
	);
	Object.assign(nodes["exec-a"], {
		friendly_name: "Start workflow",
		start: true,
	});
	Reflect.deleteProperty(nodes["exec-a"].pins, "exec-a.in");
	Object.assign(nodes["exec-b"], {
		friendly_name: "Long running task",
		long_running: true,
	});
	Object.assign(nodes["ref-a"], { friendly_name: "Function reference" });
	if (nodes["ref-a"].fn_refs) nodes["ref-a"].fn_refs.fn_refs = ["ref-b"];
	Object.assign(nodes["ref-b"], { friendly_name: "Referenced function" });
	const clean = structuredClone(nodes["string-b"]);
	clean.id = "native";
	clean.friendly_name = "Native node";
	clean.icon = undefined;
	clean.coordinates = [1060, 380, 0];
	clean.pins = Object.fromEntries(
		Object.values(clean.pins).map((p) => {
			const pin = {
				...p,
				id: p.id.replace("string-b", "native"),
				depends_on: p.pin_type === "Input" ? ["string-c.out"] : [],
				connected_to: [],
			};
			return [pin.id, pin];
		}),
	);
	nodes.native = clean;
	nodes["string-c"].pins["string-c.out"].connected_to = ["native.in"];
	clean.pins["native.out"].connected_to = ["string-a.in"];
	nodes["string-a"].pins["string-a.in"].depends_on = ["native.out"];
	const child = {
		...structuredClone(nodes["integer-b"]),
		id: "layer-child",
		layer: "group",
		coordinates: [260, 100, 0],
		pins: {},
		friendly_name: "Inside layer",
	};
	nodes[child.id] = child;
	return {
		id: "parity",
		name: "Production component parity",
		description: "",
		created_at: time,
		updated_at: time,
		execution_mode: IExecutionMode.Local,
		log_level: ILogLevel.Debug,
		stage: IExecutionStage.Dev,
		version: [0, 0, 1],
		viewport: [20, 20, 1],
		page_ids: [],
		variables: {},
		refs: {},
		nodes,
		layers: {
			group: {
				id: "group",
				name: "Open this layer",
				type: ILayerType.Collapsed,
				coordinates: [760, 590, 0],
				nodes: { [child.id]: child },
				comments: {},
				variables: {},
				pins: {},
			},
		},
		comments: {
			background: {
				id: "background",
				comment_type: ICommentType.Text,
				content:
					"## Background comment\n\nNodes and wires remain above this note.",
				coordinates: [40, 65, 0],
				width: 620,
				height: 185,
				color: "#a16207",
				z_index: 1,
				timestamp: time,
			},
			foreground: {
				id: "foreground",
				comment_type: ICommentType.Text,
				content: "Foreground comment",
				coordinates: [745, 450, 0],
				width: 210,
				height: 80,
				color: "#0d9488",
				z_index: 25,
				timestamp: time,
			},
			middle: {
				id: "middle",
				comment_type: ICommentType.Text,
				content: "Between edges and nodes",
				coordinates: [280, 450, 0],
				width: 110,
				height: 70,
				color: "#0d948880",
				z_index: 19,
				timestamp: time,
			},
			media: {
				id: "media",
				comment_type: ICommentType.Image,
				content: "Fixture image",
				presigned_url: icon,
				coordinates: [1060, 110, 0],
				width: 200,
				height: 150,
				z_index: 1,
				timestamp: time,
			},
		},
	};
}
export function applyParityCommands(
	board: IBoard,
	commands: IGenericCommand[],
) {
	for (const command of commands) {
		const c = command;
		if (command.command_type === ICommandType.UpsertComment && c.comment) {
			board.comments[c.comment.id] = structuredClone(c.comment);
			continue;
		}
		if (command.command_type === ICommandType.RemoveComment) {
			if (c.comment) Reflect.deleteProperty(board.comments, c.comment.id);
			continue;
		}
		if (command.command_type === ICommandType.UpsertPin && c.node_id && c.pin) {
			const node = board.nodes[c.node_id];
			if (node) node.pins[c.pin.id] = structuredClone(c.pin);
			continue;
		}
		applyCommands(board.nodes, [command]);
	}
}
