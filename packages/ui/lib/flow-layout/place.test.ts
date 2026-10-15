import { describe, expect, test } from "bun:test";
import { computeFlowLayoutDetailed } from "./index";
import { measureNodeBox } from "./measure";
import { GraphBuilder } from "./test-fixtures";
import { type AutoLayoutInput, getStyleConfig } from "./types";

function separatedBranches(inputHeight = 43): AutoLayoutInput {
	const graph = new GraphBuilder();
	graph.exec("root", {
		start: true,
		execIn: false,
		execOuts: ["upper", "middle", "lower"],
	});
	graph.exec("middle");
	graph.execLink("root", "middle", "middle");
	for (const branch of ["upper", "lower"]) {
		graph.exec(`${branch}-start`);
		graph.exec(`${branch}-next`);
		graph.exec(`${branch}-consumer`, { dataIns: 2 });
		graph.execLink("root", `${branch}-start`, branch);
		graph.execLink(`${branch}-start`, `${branch}-next`);
		graph.execLink(`${branch}-next`, `${branch}-consumer`);
	}
	for (let index = 0; index < 2; index++) {
		graph.pure(`input-${index}`);
		graph.dataLink(`input-${index}`, "upper-consumer", 0, index);
	}
	return graph.build({
		nodeSizes: new Map([
			["middle", [150, 500]],
			["input-0", [150, inputHeight]],
			["input-1", [150, inputHeight]],
		]),
	});
}

function y(positions: Map<string, [number, number]>, id: string): number {
	const position = positions.get(id);
	if (!position) throw new Error(`Missing position for ${id}`);
	return position[1];
}

describe("pure cluster placement", () => {
	for (const style of ["compact", "balanced", "expanded", "routed"] as const) {
		test(`uses room beside its consumer before passing another branch (${style})`, () => {
			const input = separatedBranches();
			const result = computeFlowLayoutDetailed(input, style);
			const cfg = getStyleConfig(style);
			const positions = result.positions;
			expect(result.diagnostics.columns.get("input-0")).toBe(
				result.diagnostics.columns.get("lower-next"),
			);
			expect(y(positions, "input-0")).toBeGreaterThanOrEqual(
				y(positions, "upper-next") + 43 + cfg.vGap,
			);
			expect(y(positions, "input-1") - y(positions, "input-0")).toBe(
				43 + cfg.vGap,
			);
			expect(y(positions, "input-1") + 43 + cfg.vGap).toBeLessThanOrEqual(
				y(positions, "lower-next"),
			);

			const reordered = {
				...input,
				layerNodes: [...input.layerNodes].reverse().map((node) => ({
					...node,
					coordinates: [...(positions.get(node.id) ?? [0, 0]), 0],
				})),
			};
			expect(computeFlowLayoutDetailed(reordered, style).positions).toEqual(
				positions,
			);
		});
	}

	test("keeps a cluster together when the space between branches is too small", () => {
		const input = separatedBranches(400);
		const { positions } = computeFlowLayoutDetailed(input, "compact");
		const lower = input.layerNodes.find((node) => node.id === "lower-next");
		if (!lower) throw new Error("Missing lower branch");
		expect(y(positions, "input-0")).toBe(
			y(positions, "lower-next") + measureNodeBox(lower).height + 40,
		);
		expect(y(positions, "input-1") - y(positions, "input-0")).toBe(440);
	});
});
