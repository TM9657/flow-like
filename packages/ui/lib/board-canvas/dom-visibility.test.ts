import { describe, expect, test } from "bun:test";
import type { InternalNode, Node } from "@xyflow/react";
import {
	WARM_EDITOR_LIMIT,
	equalVisibleIds,
	visibleDomIds,
	warmEditorIds,
} from "./dom-visibility";

function node(id: string, x: number, y: number, measured = true): InternalNode {
	return {
		id,
		data: {},
		position: { x, y },
		measured: measured ? { width: 180, height: 80 } : {},
		internals: { positionAbsolute: { x, y }, z: 0, userNode: {} as Node },
	};
}
const a = node("a", 100, 100);
const b = node("b", 10000, 100);
const c = node("c", -1000, 100);
const options = {
	nodes: [a, b, c],
	edges: [],
	nodeLookup: new Map([a, b, c].map((node) => [node.id, node])),
	transform: [0, 0, 1] as const,
	width: 1000,
	height: 800,
};
describe("standalone rich-content visibility", () => {
	test("keeps at most256 ordinary editors warm and excludes media", () => {
		const editors = Array.from({ length: WARM_EDITOR_LIMIT }, (_, index) => ({
			...node(String(index), 10000, 0),
			type: index % 2 ? "flowNode" : "node",
		}));
		const media = { ...node("media", 10000, 0), type: "media" };
		const retained = warmEditorIds([...editors, media]);
		expect(retained.size).toBe(WARM_EDITOR_LIMIT);
		expect(retained.has("media")).toBe(false);
		expect(
			visibleDomIds({
				...options,
				nodes: editors,
				nodeLookup: new Map(editors.map((node) => [node.id, node])),
				retained,
			}).nodes,
		).toHaveLength(WARM_EDITOR_LIMIT);
	});
	test("larger editor populations return to viewport culling without stale warm IDs", () => {
		const editors = Array.from(
			{ length: WARM_EDITOR_LIMIT + 1 },
			(_, index) => ({
				...node(String(index), index ? 10000 : 0, 0),
				type: "node",
			}),
		);
		const retained = warmEditorIds(editors);
		expect(retained.size).toBe(0);
		expect(
			visibleDomIds({
				...options,
				nodes: editors,
				nodeLookup: new Map(editors.map((node) => [node.id, node])),
				retained,
			}).nodes,
		).toEqual(["0"]);
		expect(warmEditorIds(editors.slice(1)).size).toBe(WARM_EDITOR_LIMIT);
	});
	test("culls distant rich nodes while retaining crossing SVG edges", () => {
		expect(
			visibleDomIds({
				...options,
				edges: [{ id: "cross", source: "c", target: "b" }],
			}),
		).toEqual({ nodes: ["a"], edges: ["cross"] });
	});
	test("camera changes expose cached geometry without needing mounted wrappers", () => {
		expect(
			visibleDomIds({ ...options, transform: [-10000, 0, 1] }).nodes,
		).toEqual(["b"]);
	});
	test("keeps selected, active editors, and unmeasured nodes available", () => {
		const unknown = node("unknown", 30000, 0, false);
		expect(
			visibleDomIds({
				...options,
				nodes: [a, { ...b, selected: true }, c, unknown],
				nodeLookup: new Map([...options.nodeLookup, [unknown.id, unknown]]),
				retained: new Set(["c"]),
			}).nodes,
		).toEqual(["a", "b", "c", "unknown"]);
	});
	test("unchanged visible IDs reuse the camera subscription snapshot", () => {
		const before = visibleDomIds(options);
		const after = visibleDomIds({ ...options, transform: [10, 10, 1] });
		expect(equalVisibleIds(before, after)).toBe(true);
		expect(
			equalVisibleIds(
				before,
				visibleDomIds({ ...options, transform: [-10000, 0, 1] }),
			),
		).toBe(false);
	});
});
