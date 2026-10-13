import { type MdRules, defaultRules } from "@platejs/markdown";
import { ElementApi } from "platejs";

function columnAlignment(value: unknown): "left" | "center" | "right" | null {
	return value === "left" || value === "center" || value === "right"
		? value
		: null;
}

/** Plate's default table rule preserves rich cells but drops GFM column alignment. */
export const tableMarkdown: MdRules = {
	table: {
		deserialize(node, decoration, options) {
			const deserialize = defaultRules.table?.deserialize;
			if (!deserialize)
				throw new Error("Markdown table deserializer is unavailable");
			const parsed = deserialize(node, decoration, options);
			if (!node.align?.some((align) => columnAlignment(align))) return parsed;
			return {
				...parsed,
				children: parsed.children.map((row) =>
					ElementApi.isElement(row)
						? {
								...row,
								children: row.children.map((cell, index) => {
									const align = columnAlignment(node.align?.[index]);
									return align ? { ...cell, align } : cell;
								}),
							}
						: row,
				),
			};
		},
		serialize(node, options) {
			const serialize = defaultRules.table?.serialize;
			if (!serialize)
				throw new Error("Markdown table serializer is unavailable");
			const parsed = serialize(node, options);
			const header = node.children[0];
			const align = ElementApi.isElement(header)
				? header.children.map((cell) => columnAlignment(cell.align))
				: undefined;
			return align?.some(Boolean) ? { ...parsed, align } : parsed;
		},
	},
};
