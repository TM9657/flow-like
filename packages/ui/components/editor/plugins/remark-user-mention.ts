import type { Root, Text } from "mdast";
import { visit } from "unist-util-visit";

/**
 * Remark plugin to transform user mention tags to links.
 * Converts <user>sub-id</user> to [sub-id](user://sub-id) markdown links
 * that can later be transformed to user_mention elements.
 */
export function remarkUserMention() {
	return (tree: Root) => {
		visit(tree, "text", (node: Text, index, parent) => {
			if (!parent || index === undefined) return;

			const regex = /<user>([^<]+)<\/user>/g;
			const text = node.value;

			if (!regex.test(text)) return;

			regex.lastIndex = 0;
			const parts: (Text | { type: "link"; url: string; children: Text[] })[] =
				[];
			let lastIndex = 0;
			let match: RegExpExecArray | null;

			while ((match = regex.exec(text)) !== null) {
				if (match.index > lastIndex) {
					parts.push({
						type: "text",
						value: text.slice(lastIndex, match.index),
					} as Text);
				}

				const sub = match[1].trim();
				parts.push({
					type: "link",
					url: `user://${sub}`,
					children: [{ type: "text", value: sub } as Text],
				});

				lastIndex = regex.lastIndex;
			}

			if (lastIndex < text.length) {
				parts.push({
					type: "text",
					value: text.slice(lastIndex),
				} as Text);
			}

			if (parts.length > 0) {
				parent.children.splice(index, 1, ...parts);
			}
		});
	};
}
