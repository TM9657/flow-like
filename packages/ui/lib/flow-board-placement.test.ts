import { describe, expect, test } from "bun:test";
import { handlePlaceNode } from "./flow-board-helpers";
import type { ILayer } from "./schema/flow/board";
import type { IGenericCommand } from "./schema/flow/board/commands/generic-command";
import type { INode } from "./schema/flow/node";
import {
	type IPin,
	IPinType,
	IValueType,
	IVariableType,
} from "./schema/flow/pin";

function node(id: string, type = IVariableType.String): INode {
	return {
		id,
		name: id,
		friendly_name: id,
		description: "",
		category: "test",
		pins: Object.fromEntries(
			[IPinType.Input, IPinType.Output].map((direction) => {
				const pin: IPin = {
					id: `${id}.${direction}`,
					name: direction,
					friendly_name: direction,
					description: "",
					index: 0,
					pin_type: direction,
					data_type: type,
					value_type: IValueType.Normal,
					connected_to: [],
					depends_on: [],
				};
				return [pin.id, pin];
			}),
		),
		fn_refs: {
			fn_refs: [],
			can_reference_fns: true,
			can_be_referenced_by_fns: true,
		},
	};
}
function fixture(type = IVariableType.String) {
	const source = node("source", type);
	const occupied = node("occupied", type);
	const template = node("template", type);
	source.pins["source.Output"].connected_to = ["occupied.Input"];
	occupied.pins["occupied.Input"].depends_on = ["source.Output"];
	const boardNodes = { source, occupied };
	const pinCache = new Map<string, [IPin, INode | ILayer, boolean]>(
		Object.values(boardNodes).flatMap((n) =>
			Object.values(n.pins).map(
				(p) =>
					[p.id, [p, n, true]] as [string, [IPin, INode | ILayer, boolean]],
			),
		),
	);
	const batches: IGenericCommand[][] = [];
	return {
		source,
		occupied,
		template,
		batches,
		args: {
			node: template,
			position: { x: 120, y: 45 },
			droppedPin: source.pins["source.Output"],
			currentLayer: undefined,
			refs: {},
			boardNodes,
			pinCache,
			executeCommands: async (commands: IGenericCommand[]) => {
				batches.push(structuredClone(commands));
			},
		},
	};
}
describe("palette node placement", () => {
	test("creates and connects once without changing the catalog's pin identities", async () => {
		const f = fixture();
		const before = structuredClone(f.template);
		await handlePlaceNode(f.args);
		expect(f.template).toEqual(before);
		expect(f.batches).toHaveLength(1);
		expect(f.batches[0].map((c) => String(c.command_type))).toEqual([
			"AddNode",
			"ConnectPin",
		]);
		const added = f.batches[0][0].node;
		expect(added?.coordinates).toEqual([120, 45, 0]);
		expect(f.batches[0][1].to_node).toBe(added?.id);
	});
	test("an occupied execution output is explicitly disconnected in the same undo batch", async () => {
		const f = fixture(IVariableType.Execution);
		await handlePlaceNode(f.args);
		expect(f.batches[0].map((c) => String(c.command_type))).toEqual([
			"AddNode",
			"DisconnectPin",
			"ConnectPin",
		]);
		expect(f.batches[0][1]).toMatchObject({
			from_node: "source",
			to_node: "occupied",
		});
	});
	test("backward data placement preserves the displaced source for undo", async () => {
		const f = fixture();
		f.args.droppedPin = f.occupied.pins["occupied.Input"];
		await handlePlaceNode(f.args);
		expect(f.batches[0].map((c) => String(c.command_type))).toEqual([
			"AddNode",
			"DisconnectPin",
			"ConnectPin",
		]);
		expect(f.batches[0][2].to_node).toBe("occupied");
	});
	test("function-reference placement retains source flags and shares the add transaction", async () => {
		const f = fixture();
		f.args.droppedPin = { ...f.args.droppedPin, id: "ref_out_source" };
		await handlePlaceNode(f.args);
		expect(f.batches[0].map((c) => String(c.command_type))).toEqual([
			"AddNode",
			"UpdateNode",
		]);
		expect(f.batches[0][1].node?.fn_refs).toEqual({
			...f.source.fn_refs,
			fn_refs: [f.batches[0][0].node?.id],
		});
	});
	test("stale dropped pins do not leave an orphan node", async () => {
		const f = fixture();
		f.args.pinCache.delete(f.args.droppedPin.id);
		await expect(handlePlaceNode(f.args)).rejects.toThrow(
			"no longer available",
		);
		expect(f.batches).toHaveLength(0);
	});
	test("backend rejection leaves the template reusable", async () => {
		const f = fixture();
		const before = structuredClone(f.template);
		f.args.executeCommands = async () => {
			throw new Error("rejected");
		};
		await expect(handlePlaceNode(f.args)).rejects.toThrow("rejected");
		expect(f.template).toEqual(before);
	});
});
