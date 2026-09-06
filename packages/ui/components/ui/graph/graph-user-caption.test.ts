import { describe, expect, test } from "bun:test";
import Graph from "graphology";
import type {
	GraphOverlay,
	SubgraphNode,
} from "../../../state/backend-state/graph-state";
import { normalizeGraphQueryResult } from "../../../state/backend-state/graph-state";
import {
	applyNodeCaptionLabels,
	nodeCaptionAccountId,
} from "./graph-user-caption";

const SUB = "42c52474-5081-70d7-2b23-4bd8c38d8fb0";
const node: SubgraphNode = {
	id: `Person:${SUB}`,
	label: "Person",
	caption: SUB,
	props: { sub: SUB, name: "Felix Schultz", board_id: SUB },
};
const overlay = {
	nodes: [{ label: "Person", id_column: "sub" }],
	object_views: [],
} as unknown as GraphOverlay;

describe("ontology account captions", () => {
	test("uses the mapped account key and preserves explicit display fields", () => {
		expect(nodeCaptionAccountId(node, overlay)).toBe(SUB);
		expect(nodeCaptionAccountId(node)).toBe(SUB);
		expect(
			nodeCaptionAccountId(node, {
				...overlay,
				nodes: [{ ...overlay.nodes[0], display_column: "name" }],
			}),
		).toBeNull();
	});

	test("respects object-view titles and avoids treating every opaque ID as a person", () => {
		expect(
			nodeCaptionAccountId(node, {
				...overlay,
				object_views: [
					{
						object_type: "Person",
						title_property: "name",
						prominent_properties: [],
					},
				],
			}),
		).toBeNull();
		expect(
			nodeCaptionAccountId(node, {
				...overlay,
				nodes: [{ ...overlay.nodes[0], id_column: "board_id" }],
			}),
		).toBeNull();
		expect(
			nodeCaptionAccountId(
				{ ...node, props: { sub: "system" }, caption: "system" },
				overlay,
			),
		).toBeNull();
	});

	test("canvas captions resolve their stored account even when the inspector has a different title", () => {
		const customized = {
			...overlay,
			object_views: [
				{
					object_type: "Person",
					title_property: "name",
					prominent_properties: [],
				},
			],
		};
		expect(nodeCaptionAccountId(node, customized)).toBeNull();
		expect(nodeCaptionAccountId(node, customized, false)).toBe(SUB);
	});

	test("canvas label updates preserve IDs, positions, properties and stored captions", () => {
		const graph = new Graph();
		graph.addNode(node.id, { label: SUB, x: 10, y: 20, props: node.props });
		let updates = 0;
		graph.on("nodeAttributesUpdated", () => {
			updates++;
		});
		const labels = new Map([[node.id, "Felix Schultz"]]);
		applyNodeCaptionLabels(graph, [node], labels);
		expect(graph.getNodeAttribute(node.id, "label")).toBe("Felix Schultz");
		expect(graph.getNodeAttribute(node.id, "x")).toBe(10);
		expect(graph.getNodeAttribute(node.id, "y")).toBe(20);
		expect(graph.getNodeAttribute(node.id, "props")).toBe(node.props);
		expect(node.caption).toBe(SUB);
		expect(graph.nodes()).toEqual([node.id]);
		applyNodeCaptionLabels(graph, [node], labels);
		expect(updates).toBe(1);
		applyNodeCaptionLabels(graph, [node], new Map());
		expect(graph.getNodeAttribute(node.id, "label")).toBe(SUB);
	});
});

test("query metadata is optional for older backends", () => {
	const rows = [{ "n.sub": SUB }];
	expect(normalizeGraphQueryResult(rows)).toEqual({
		rows,
		property_metadata: {},
	});
	const result = {
		rows,
		property_metadata: {
			"n.location": { "ARROW:extension:name": "geoarrow.wkb" },
		},
	};
	expect(normalizeGraphQueryResult(result)).toBe(result);
});
