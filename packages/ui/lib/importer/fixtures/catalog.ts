import type { INode } from "../../schema";
import { createNode, createPin } from "../board-builder";
import { BPMN_NODE_SPECS, type FallbackNodeSpec } from "../bpmn-nodes";

/**
 * A stand-in for `boardState.getCatalog` covering every node the BPMN
 * translator places. Built from the same specs the translator falls back to,
 * but with catalog-side ids, versions and descriptions, so a test can tell
 * "cloned from the catalog" apart from "minted from the fallback table".
 */
export function bpmnCatalogFixture(exclude: readonly string[] = []): INode[] {
	return Object.entries(BPMN_NODE_SPECS)
		.filter(([name]) => !exclude.includes(name))
		.map(([name, spec], index) => catalogNode(name, spec, 10 + index));
}

function catalogNode(
	name: string,
	spec: FallbackNodeSpec,
	version: number,
): INode {
	const node = createNode({
		name,
		friendlyName: spec.friendly,
		description: `Catalog ${name}`,
		category: spec.category,
		x: 0,
		y: 0,
		start: spec.start,
	});
	node.version = version;
	for (const pin of spec.pins) {
		const created = createPin({
			name: pin.name,
			friendlyName: pin.friendly,
			description: `catalog pin ${pin.name}`,
			pinType: pin.type,
			dataType: pin.data,
			valueType: pin.value,
			defaultValue: pin.default,
		});
		node.pins[created.id] = created;
	}
	return node;
}
