import { expect, test } from "bun:test";
import type { INode } from "../schema/flow/node";
import { addNodeCommand } from "./generic-command";

test("placing catalog nodes never rewrites the shared template or previous placement", () => {
	const template = {
		id: "catalog",
		name: "test",
		pins: {
			input: {
				id: "input",
				default_value: [1, 2],
				connected_to: [],
				depends_on: [],
			},
		},
	} as unknown as INode;
	Object.freeze(template.pins.input);
	Object.freeze(template.pins);
	Object.freeze(template);
	const first = addNodeCommand({ node: template });
	const second = addNodeCommand({ node: template });
	expect(template.id).toBe("catalog");
	expect(Object.keys(template.pins)).toEqual(["input"]);
	expect(first.node.id).not.toBe(second.node.id);
	const firstPin = Object.values(first.node.pins)[0];
	const secondPin = Object.values(second.node.pins)[0];
	expect(firstPin.id).not.toBe(secondPin.id);
	firstPin.default_value!.push(3);
	expect(secondPin.default_value).toEqual([1, 2]);
	expect(template.pins.input.default_value).toEqual([1, 2]);
	expect((first.command as { node?: INode }).node).toBe(first.node);
});
