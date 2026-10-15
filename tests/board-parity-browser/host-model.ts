import { createId } from "@paralleldrive/cuid2";
import type { IBoard } from "../../packages/ui/lib/schema/flow/board";
import {
	ICommandType as C,
	type IGenericCommand,
} from "../../packages/ui/lib/schema/flow/board/commands/generic-command";
import { applyCommands } from "../board-canvas-browser/model";

/** Local backend receipts for the real host. Undo consumes the same fields as Rust commands. */
export function applyHostCommand(
	board: IBoard,
	input: IGenericCommand,
	inverse = false,
): IGenericCommand {
	const c = structuredClone(input);
	const entity = (id: string) =>
		board.nodes[id] ?? board.comments[id] ?? board.layers[id];
	if (c.command_type === C.MoveNode && c.node_id) {
		const value = entity(c.node_id);
		if (!value) throw new Error(`Unknown moved entity ${c.node_id}`);
		const previous = [...(value.coordinates ?? [0, 0, 0])];
		value.coordinates = [
			...((inverse ? c.from_coordinates : c.to_coordinates) ?? previous),
		];
		return { ...c, from_coordinates: previous };
	}
	if (c.command_type === C.AddNode && c.node) {
		if (inverse) delete board.nodes[c.node.id];
		else board.nodes[c.node.id] = { ...c.node, layer: c.current_layer };
		return c;
	}
	if (c.command_type === C.RemoveNode && c.node) {
		if (inverse) {
			board.nodes[c.node.id] = c.node;
			for (const node of c.connected_nodes ?? []) board.nodes[node.id] = node;
		} else {
			const removed = board.nodes[c.node.id];
			if (!removed) throw new Error(`Unknown deleted entity ${c.node.id}`);
			c.node = structuredClone(removed);
			c.connected_nodes = [];
			const pins = new Set(Object.keys(removed.pins));
			for (const node of Object.values(board.nodes)) {
				if (node.id === removed.id) continue;
				if (
					Object.values(node.pins).some((p) =>
						[...p.connected_to, ...p.depends_on].some((id) => pins.has(id)),
					) ||
					node.fn_refs?.fn_refs.includes(removed.id)
				) {
					c.connected_nodes.push(structuredClone(node));
					for (const pin of Object.values(node.pins)) {
						pin.connected_to = pin.connected_to.filter((id) => !pins.has(id));
						pin.depends_on = pin.depends_on.filter((id) => !pins.has(id));
					}
					if (node.fn_refs)
						node.fn_refs.fn_refs = node.fn_refs.fn_refs.filter(
							(id: string) => id !== removed.id,
						);
				}
			}
			delete board.nodes[removed.id];
		}
		return c;
	}
	if (c.command_type === C.UpdateNode && c.node) {
		const old_node = structuredClone(board.nodes[c.node.id]);
		if (inverse && c.old_node) board.nodes[c.old_node.id] = c.old_node;
		else board.nodes[c.node.id] = c.node;
		return { ...c, old_node };
	}
	if (
		(c.command_type === C.UpsertComment ||
			c.command_type === C.RemoveComment) &&
		c.comment
	) {
		const old_comment = board.comments[c.comment.id];
		if (
			c.command_type === C.RemoveComment ? !inverse : inverse && !c.old_comment
		)
			delete board.comments[c.comment.id];
		else
			board.comments[c.comment.id] =
				inverse && c.command_type === C.UpsertComment && c.old_comment
					? c.old_comment
					: { ...c.comment, layer: c.current_layer ?? c.comment.layer };
		return { ...c, old_comment };
	}
	if (c.command_type === C.UpsertLayer && c.layer) {
		const old_layer = board.layers[c.layer.id];
		if (inverse) {
			if (c.old_layer) board.layers[c.layer.id] = c.old_layer;
			else delete board.layers[c.layer.id];
			for (const value of [
				...Object.values(board.nodes),
				...Object.values(board.comments),
			])
				if (value.layer === c.layer.id)
					value.layer = c.old_layer?.id ?? c.current_layer;
		} else {
			board.layers[c.layer.id] = {
				...c.layer,
				parent_id: old_layer?.parent_id ?? c.current_layer,
			};
			for (const id of c.node_ids ?? []) {
				const value = entity(id);
				if (value) value.layer = c.layer.id;
			}
		}
		return { ...c, old_layer };
	}
	if (c.command_type === C.RemoveLayer && c.layer && c.preserve_nodes) {
		if (inverse) {
			board.layers[c.layer.id] = c.layer;
			for (const id of c.layer_nodes ?? []) {
				const value = entity(id);
				if (value) value.layer = c.layer.id;
			}
		} else {
			c.layer = board.layers[c.layer.id];
			c.layer_nodes = [];
			for (const value of [
				...Object.values(board.nodes),
				...Object.values(board.comments),
			])
				if (value.layer === c.layer.id) {
					c.layer_nodes.push(value.id);
					value.layer = c.layer.parent_id;
				}
			delete board.layers[c.layer.id];
		}
		return c;
	}

	if (c.command_type === C.MoveToLayer) {
		const previous: Record<string, string | null> = {};
		for (const id of c.ids ?? []) {
			const value = entity(id);
			if (!value) continue;
			previous[id] = value.layer ?? null;
			value.layer = inverse ? c.previous?.[id] : c.target;
		}
		return { ...c, previous };
	}
	if (c.command_type === C.CopyPaste) {
		if (inverse) {
			for (const n of c.new_nodes ?? []) delete board.nodes[n.id];
			for (const n of c.new_comments ?? []) delete board.comments[n.id];
			for (const n of c.new_layers ?? []) delete board.layers[n.id];
			for (const id of c.added_refs ?? []) delete board.refs[id];
			for (const id of c.added_variables ?? []) delete board.variables[id];
			return c;
		}
		if (
			!(c.new_nodes?.length || c.new_comments?.length || c.new_layers?.length)
		) {
			const ids = new Map<string, string>();
			for (const value of [
				...(c.original_nodes ?? []),
				...(c.original_comments ?? []),
				...(c.original_layers ?? []),
			])
				ids.set(value.id, createId());
			for (const node of c.original_nodes ?? [])
				for (const pin of Object.values(node.pins)) ids.set(pin.id, createId());
			const mappedId = (id: string) => {
				const mapped = ids.get(id);
				if (!mapped) throw new Error(`Missing copied id ${id}`);
				return mapped;
			};
			const old = c.old_mouse ??
				c.original_nodes?.[0]?.coordinates ??
				c.original_comments?.[0]?.coordinates ?? [0, 0, 0];
			const coords = (value?: number[] | null) =>
				[0, 1, 2].map(
					(i) => (value?.[i] ?? 0) + (c.offset?.[i] ?? 0) - (old[i] ?? 0),
				);
			c.new_nodes = (c.original_nodes ?? []).map((node) => ({
				...node,
				id: mappedId(node.id),
				layer: ids.get(node.layer ?? "") ?? c.current_layer,
				coordinates: coords(node.coordinates),
				pins: Object.fromEntries(
					Object.values(node.pins).map((pin) => {
						const p = {
							...pin,
							id: mappedId(pin.id),
							connected_to: pin.connected_to.flatMap((id) => ids.get(id) ?? []),
							depends_on: pin.depends_on.flatMap((id) => ids.get(id) ?? []),
						};
						return [p.id, p];
					}),
				),
				fn_refs: node.fn_refs
					? {
							...node.fn_refs,
							fn_refs: node.fn_refs.fn_refs.flatMap(
								(id: string) => ids.get(id) ?? [],
							),
						}
					: undefined,
			}));
			c.new_comments = (c.original_comments ?? []).map((comment) => ({
				...comment,
				id: mappedId(comment.id),
				layer: ids.get(comment.layer ?? "") ?? c.current_layer,
				coordinates: coords(comment.coordinates),
			}));
			c.new_layers = (c.original_layers ?? []).map((layer) => ({
				...layer,
				id: mappedId(layer.id),
				parent_id: ids.get(layer.parent_id ?? "") ?? c.current_layer,
				coordinates: coords(layer.coordinates),
			}));
		}
		for (const n of c.new_nodes ?? []) board.nodes[n.id] = n;
		for (const n of c.new_comments ?? []) board.comments[n.id] = n;
		for (const n of c.new_layers ?? []) board.layers[n.id] = n;
		c.added_refs = Object.keys(c.original_refs ?? {}).filter(
			(key) => !(key in board.refs),
		);
		for (const key of c.added_refs) board.refs[key] = c.original_refs[key];
		c.added_variables = (c.original_variables ?? [])
			.filter((v: { id: string }) => !(v.id in board.variables))
			.map((v: { id: string }) => v.id);
		for (const v of c.original_variables ?? [])
			if (c.added_variables.includes(v.id)) board.variables[v.id] = v;
		return c;
	}
	if (c.command_type === C.ConnectPin || c.command_type === C.DisconnectPin)
		return applyCommands(board.nodes, [
			{
				...c,
				command_type: inverse
					? c.command_type === C.ConnectPin
						? C.DisconnectPin
						: C.ConnectPin
					: c.command_type,
			},
		])[0];
	throw new Error(`Unsupported host fixture command ${c.command_type}`);
}
