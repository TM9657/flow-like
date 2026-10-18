import { afterAll, afterEach, describe, expect, test } from "bun:test";
import type { IPin } from "../../../../lib/schema/flow/pin";
import { parseUint8ArrayToJson } from "../../../../lib/uint8";
import {
	allByRole,
	byRole,
	click,
	installDom,
	typeInto,
} from "../../../settings/devices/testing/dom-harness";
import {
	type FlowElementOption,
	createEmptyFlowSelectorData,
} from "../../flow-selector-data";

const dom = installDom();
const { ElementSelect } = await import("./element-select");

afterEach(dom.cleanup);
afterAll(dom.restore);

async function openSelector(elements: FlowElementOption[] = []) {
	const selected: unknown[] = [];
	const { container } = await dom.render(
		<ElementSelect
			pin={{ friendly_name: "Editor Element" } as IPin}
			value={undefined}
			setValue={(value) =>
				selected.push(parseUint8ArrayToJson(value as number[]))
			}
			selectorDataRef={{
				current: {
					...createEmptyFlowSelectorData(),
					elementOptions: elements,
					loadElements: async () => elements,
				},
			}}
		/>,
	);
	const trigger = byRole("button", "Select element", container);
	await click(trigger);
	return { selected, trigger, input: byRole("combobox") };
}

describe("element reference picker", () => {
	test("commits a copied Storage target and resets search when reopened", async () => {
		const reference = "storage:app:user/folder/Assembly 01.solidcraft";
		const { selected, trigger, input } = await openSelector();
		await typeInto(input, reference);
		await click(byRole("option", /^Use reference/));
		expect(selected).toEqual([reference]);
		expect(allByRole("combobox")).toHaveLength(0);
		await click(trigger);
		expect((byRole("combobox") as HTMLInputElement).value).toBe("");
		expect(allByRole("option")).toHaveLength(0);
	});

	test("keeps qualified page selection when searching by full reference or page name", async () => {
		const { selected, trigger, input } = await openSelector([
			{
				id: "surface-a/preview",
				rawId: "preview",
				type: "filePreview",
				label: "preview",
				pageName: "Assembly",
			},
			{
				id: "surface-b/preview",
				rawId: "preview",
				type: "filePreview",
				label: "preview",
				pageName: "Drawing",
			},
		]);
		await typeInto(input, "surface-b/preview");
		expect(allByRole("option")).toHaveLength(1);
		expect(allByRole("option", /^Use reference/)).toHaveLength(0);
		await click(byRole("option", /Drawing/));
		await click(trigger);
		await typeInto(byRole("combobox"), "Assembly");
		await click(byRole("option", /Assembly/));
		expect(selected).toEqual(["surface-b/preview", "surface-a/preview"]);
	});

	test("limits manual references by UTF-8 bytes", async () => {
		const reference = `s/${"é".repeat(511)}`;
		const { selected, input } = await openSelector();
		await typeInto(input, `${reference}x`);
		expect(allByRole("option")).toHaveLength(0);
		await typeInto(input, reference);
		await click(byRole("option", /^Use reference/));
		expect(selected).toEqual([reference]);
	});

	test.each([
		"preview",
		"/preview",
		"surface/",
		"surface/part\u0001",
		"surface/part\u0085",
	])("does not offer an invalid manual reference: %j", async (reference) => {
		const { selected, input } = await openSelector();
		await typeInto(input, reference);
		expect(allByRole("option")).toHaveLength(0);
		expect(selected).toEqual([]);
	});
});
