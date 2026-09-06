import type Graph from "graphology";
import { looksLikeUserColumnName } from "../../../lib/user-display";
import type {
	GraphOverlay,
	SubgraphNode,
} from "../../../state/backend-state/graph-state";
import { accountIdFromValue } from "../../../state/backend-state/user-state";

/** Resolve captions only when their mapped property names an account. */
export function nodeCaptionAccountId(
	node: SubgraphNode | undefined,
	overlay?: GraphOverlay,
	useObjectTitle = true,
): string | null {
	if (!node) return null;
	const mapping = overlay?.nodes.find((entry) => entry.label === node.label);
	const view =
		mapping && useObjectTitle
			? overlay?.object_views?.find((entry) =>
					[mapping.id, mapping.api_name, mapping.label].includes(
						entry.object_type,
					),
				)
			: undefined;
	const property =
		view?.title_property ?? mapping?.display_column ?? mapping?.id_column;
	if (property) {
		return looksLikeUserColumnName(property)
			? accountIdFromValue(node.props[property] ?? node.caption)
			: null;
	}
	const entry = Object.entries(node.props).find(
		([key, value]) => looksLikeUserColumnName(key) && value === node.caption,
	);
	return entry ? accountIdFromValue(entry[1]) : null;
}

/** Update visible names without rebuilding the scene or modifying its stored records. */
export function applyNodeCaptionLabels(
	graph: Pick<Graph, "hasNode" | "getNodeAttribute" | "setNodeAttribute">,
	nodes: readonly SubgraphNode[],
	labels?: ReadonlyMap<string, string>,
): void {
	for (const node of nodes) {
		if (!graph.hasNode(node.id)) continue;
		const label = labels?.get(node.id) ?? node.caption ?? node.id;
		if (graph.getNodeAttribute(node.id, "label") !== label)
			graph.setNodeAttribute(node.id, "label", label);
	}
}
