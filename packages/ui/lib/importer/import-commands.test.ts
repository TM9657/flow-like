import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import type { IGenericCommand } from "../schema";
import { ILayerType } from "../schema/flow/board";
import { ICommandType } from "../schema/flow/board/commands/generic-command";
import { parseBpmn } from "./bpmn-model";
import { translateBpmn } from "./bpmn-translator";
import { buildImportCommands, importedModuleId } from "./import-commands";

const fixture = readFileSync(
	resolve(import.meta.dir, "fixtures", "bpmn", "collaboration.bpmn"),
	"utf-8",
);

function paste(commands: IGenericCommand[]) {
	const command = commands[commands.length - 1] as IGenericCommand &
		Record<string, unknown>;
	expect(command.command_type).toBe(ICommandType.CopyPaste);
	return command as unknown as {
		original_layers: Array<{
			id: string;
			type: ILayerType;
			parent_id: string | null;
		}>;
		original_nodes: Array<{ layer: string | null }>;
		original_comments: Array<{ layer: string | null }>;
		current_layer?: string;
		old_mouse?: number[];
	};
}

describe("buildImportCommands", () => {
	const result = translateBpmn(parseBpmn(fixture));

	test("wraps the fragment in a module and re-parents every top-level item", () => {
		const plan = buildImportCommands(result, {
			kind: "module",
			name: "Orders",
			parentId: null,
		});
		const command = paste(plan.commands);
		const module = command.original_layers.find(
			(l) => l.type === ILayerType.Module,
		);
		expect(module?.id).toBe(plan.moduleId as string);
		expect(module?.parent_id).toBeNull();
		for (const layer of command.original_layers) {
			if (layer.id === module?.id) continue;
			expect(layer.parent_id).not.toBeNull();
		}
		for (const node of command.original_nodes)
			expect(node.layer).not.toBeNull();
		for (const comment of command.original_comments) {
			expect(comment.layer).not.toBeNull();
		}
		expect(command.current_layer).toBeUndefined();
		expect(command.old_mouse).toEqual([0, 0, 0]);
	});

	test("nests the module under a parent module and targets a file directly", () => {
		const nested = paste(
			buildImportCommands(result, {
				kind: "module",
				name: "Orders",
				parentId: "parent-module",
			}).commands,
		);
		expect(nested.current_layer).toBe("parent-module");

		const direct = buildImportCommands(result, {
			kind: "layer",
			layerId: "open-module",
		});
		const command = paste(direct.commands);
		expect(direct.moduleId).toBeUndefined();
		expect(command.current_layer).toBe("open-module");
		expect(
			command.original_layers.some((l) => l.type === ILayerType.Module),
		).toBe(false);
		expect(command.original_layers.some((l) => l.parent_id === null)).toBe(
			true,
		);
	});

	test("batches an upsert for every variable ahead of the paste", () => {
		const plan = buildImportCommands(result, { kind: "layer" });
		const upserts = plan.commands.filter(
			(c) => c.command_type === ICommandType.UpsertVariable,
		);
		expect(upserts).toHaveLength(Object.keys(result.board.variables).length);
		expect(plan.commands.indexOf(upserts[upserts.length - 1])).toBeLessThan(
			plan.commands.length - 1,
		);
	});

	test("does not mutate the translation it was given", () => {
		const before = JSON.stringify(result.board);
		buildImportCommands(result, { kind: "module", name: "X", parentId: null });
		expect(JSON.stringify(result.board)).toBe(before);
	});
});

describe("importedModuleId", () => {
	test("reads the minted module off the executed paste, in a batch or alone", () => {
		const executed = {
			command_type: ICommandType.CopyPaste,
			new_layers: [
				{ id: "collapsed", type: ILayerType.Collapsed },
				{ id: "minted", type: ILayerType.Module },
			],
		} as unknown as IGenericCommand;
		expect(importedModuleId(executed)).toBe("minted");
		expect(importedModuleId([{} as IGenericCommand, executed])).toBe("minted");
		expect(importedModuleId(undefined)).toBeUndefined();
		expect(importedModuleId({} as IGenericCommand)).toBeUndefined();
	});
});
