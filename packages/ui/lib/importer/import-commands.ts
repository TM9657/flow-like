import {
	copyPasteCommand,
	upsertVariableCommand,
} from "../command/generic-command";
import { newModuleLayer } from "../flow-modules";
import type { IBoard, IGenericCommand } from "../schema";
import { ILayerType } from "../schema/flow/board";
import { ICommandType } from "../schema/flow/board/commands/generic-command";
import type { TranslationResult } from "./types";

/**
 * Where an imported fragment lands.
 *
 * `module` wraps the whole fragment in a new `.flow` module so the import
 * shows up as its own file in the explorer; `layer` drops it into the file
 * that is open (`undefined` = `main.flow`).
 */
export type ImportTarget =
	| { kind: "module"; name: string; parentId: string | null }
	| { kind: "layer"; layerId?: string };

export interface ImportCommandPlan {
	commands: IGenericCommand[];
	/** Fragment-local id of the module the paste will re-mint, if one was created. */
	moduleId?: string;
}

/** Where a fragment lands when the caller has no cursor to place it at. */
const IMPORT_OFFSET: readonly [number, number, number] = [100, 100, 0];

/**
 * One undoable batch: every board variable the translation produced (the
 * paste only restores variables a `var_ref` pin points at), then the fragment
 * itself. Always sent with `old_mouse: [0,0,0]` so coordinates translate
 * verbatim instead of re-basing on the first node.
 */
export function buildImportCommands(
	result: TranslationResult,
	target: ImportTarget,
	offset: readonly number[] = IMPORT_OFFSET,
): ImportCommandPlan {
	// The translation is memoised by the dialog; re-parenting must not leak into
	// it, or a retry with another target would reference a module that is gone.
	const board = structuredClone(result.board);
	const layers = Object.values(board.layers);
	const nodes = Object.values(board.nodes);
	const comments = Object.values(board.comments);
	const variables = Object.values(board.variables);

	let moduleId: string | undefined;
	let currentLayer: string | undefined;
	if (target.kind === "module") {
		const module = newModuleLayer(target.name, null);
		moduleId = module.id;
		for (const layer of layers) {
			if (!layer.parent_id) layer.parent_id = module.id;
		}
		for (const node of nodes) {
			if (!node.layer) node.layer = module.id;
		}
		for (const comment of comments) {
			if (!comment.layer) comment.layer = module.id;
		}
		layers.unshift(module);
		currentLayer = target.parentId ?? undefined;
	} else {
		currentLayer = target.layerId;
	}

	const commands: IGenericCommand[] = variables.map((variable) =>
		upsertVariableCommand({ variable }),
	);
	commands.push(
		copyPasteCommand({
			original_nodes: nodes,
			original_comments: comments,
			original_layers: layers,
			original_variables: variables,
			original_refs: stripInternalRefs(board),
			new_comments: [],
			new_nodes: [],
			new_layers: [],
			current_layer: currentLayer,
			old_mouse: [0, 0, 0],
			offset: [...offset],
		}),
	);
	return { commands, moduleId };
}

/** The module id the backend minted for the fragment's module, read off the executed paste. */
export function importedModuleId(
	executed: IGenericCommand | IGenericCommand[] | undefined,
): string | undefined {
	const list = Array.isArray(executed) ? executed : executed ? [executed] : [];
	for (const command of list) {
		if (command?.command_type !== ICommandType.CopyPaste) continue;
		const module = command.new_layers?.find(
			(layer) => layer.type === ILayerType.Module,
		);
		if (module) return module.id;
	}
	return undefined;
}

const INTERNAL_REF_PREFIX = "__flow_like_internal_v1/";

function stripInternalRefs(board: IBoard): Record<string, string> {
	return Object.fromEntries(
		Object.entries(board.refs ?? {}).filter(
			([key]) => !key.startsWith(INTERNAL_REF_PREFIX),
		),
	);
}
