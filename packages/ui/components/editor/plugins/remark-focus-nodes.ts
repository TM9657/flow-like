import type { Root } from "mdast";
import { visit } from "unist-util-visit";

/**
 * Remark plugin to transform focus node links
 * The preprocessing in MessageContent converts <focus_node>nodeId</focus_node>
 * to [NodeName](focus://nodeId) markdown links before this plugin runs.
 * This plugin ensures those links are properly parsed as link nodes.
 */
export function remarkFocusNodes() {
	return (tree: Root) => {
		// The links are already in standard markdown format [text](focus://id)
		// so the markdown parser handles them. We just need to ensure they exist.
		// This plugin is kept for any additional processing if needed.
		visit(tree, "link", (node: any) => {
			if (
				node.url &&
				typeof node.url === "string" &&
				node.url.startsWith("focus://")
			) {
				// Mark the link as a focus node link
				node.data = node.data || {};
				node.data.hProperties = node.data.hProperties || {};
				node.data.hProperties["data-focus-node"] = "true";
			}
		});
	};
}
