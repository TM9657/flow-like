import { describe, expect, test } from "bun:test";
import { type Edge, applyEdgeChanges } from "@xyflow/react";
import { handleEdgesChange } from "./flow-board-helpers";

describe("connection selection", () => {
	for (const type of ["data", "veil"]) {
		for (const reduceMotion of [false, true]) {
			test(`${type} dashes stay stationary through selection with reduceMotion=${reduceMotion}`, () => {
				const selected = { current: new Set<string>() };
				const original: Edge = {
					id: "connection",
					source: "from",
					target: "to",
					type,
					animated: false,
					selected: false,
					data: { reduceMotion },
				};
				let edges = [original];
				const commands: unknown[][] = [];
				for (const nextSelected of [true, false, true, false]) {
					edges = handleEdgesChange({
						changes: [
							{ id: original.id, type: "select", selected: nextSelected },
						],
						currentEdges: edges,
						selected,
						version: undefined,
						boardData: undefined,
						pinCache: new Map(),
						deletingNodesRef: { current: new Set() },
						executeCommands: async (batch) => {
							commands.push(batch);
						},
						applyEdgeChanges,
					});
					expect(edges[0]).toMatchObject({
						animated: false,
						selected: nextSelected,
						data: { reduceMotion },
					});
					expect(selected.current.has(original.id)).toBe(nextSelected);
				}
				expect(commands).toEqual([]);
				expect(original.selected).toBe(false);
			});
		}
	}
});
