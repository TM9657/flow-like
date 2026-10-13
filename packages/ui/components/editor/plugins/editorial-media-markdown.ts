import { type MdRules, defaultRules, parseAttributes } from "@platejs/markdown";
import { NodeApi } from "platejs";
import { safeStreamingImageUrl } from "../streaming-editorial-image";

/** Plain Markdown images stay portable; editorial metadata uses an inert MDX image tag. */
export const editorialMediaMarkdown: MdRules = {
	img: {
		deserialize(node: any, decoration, options): any {
			const parsed = defaultRules.img!.deserialize!(node, decoration, options);
			const attributes = node.attributes
				? parseAttributes(node.attributes)
				: {};
			const pending =
				attributes.streamingPending === true ||
				attributes.streamingPending === "true";
			if (pending) {
				// MDX attributes are text; parseAttributes otherwise coerces strings
				// such as alt="123" into JSON scalars.
				for (const attribute of node.attributes ?? [])
					if (typeof attribute.value === "string")
						attributes[attribute.name] = attribute.value;
			}
			const { focalX, focalY, caption, rightsExpiresAt, ...rest } = attributes;
			const pendingUrl = pending
				? safeStreamingImageUrl(attributes.src)
				: undefined;
			return {
				...parsed,
				...rest,
				...(pending
					? {
							url: pendingUrl,
							streamingPending:
								pendingUrl && typeof attributes.alt === "string"
									? "ready"
									: "loading",
						}
					: {}),
				...(rightsExpiresAt !== undefined
					? { rightsExpiresAt: Number(rightsExpiresAt) }
					: {}),
				alt:
					typeof attributes.alt === "string"
						? attributes.alt
						: (node.alt ?? ""),
				...(typeof caption === "string"
					? { caption: [{ text: caption }] }
					: pending
						? {
								caption: [
									{
										text:
											typeof attributes.alt === "string" ? attributes.alt : "",
									},
								],
							}
						: {}),
				...(focalX !== undefined && focalY !== undefined
					? { focalPoint: { x: Number(focalX), y: Number(focalY) } }
					: {}),
			};
		},
		serialize(node: any): any {
			const caption = Array.isArray(node.caption)
				? node.caption.map((item: any) => NodeApi.string(item)).join("\n")
				: "";
			if (
				!node.credit &&
				!node.license &&
				!node.focalPoint &&
				!node.rightsExpiresAt &&
				(!caption || caption === node.alt)
			) {
				return {
					type: "paragraph",
					children: [
						{
							type: "image",
							url: node.url,
							alt: node.alt ?? caption,
							title: node.title,
						},
					],
				};
			}
			const values: Record<string, unknown> = {
				src: node.url,
				alt: node.alt ?? "",
				caption,
				credit: node.credit,
				license: node.license,
				rightsExpiresAt: node.rightsExpiresAt,
				focalX: node.focalPoint?.x,
				focalY: node.focalPoint?.y,
			};
			return {
				type: "mdxJsxFlowElement",
				name: "img",
				children: [],
				attributes: Object.entries(values)
					.filter(([, value]) => value !== undefined)
					.map(([name, value]) => ({
						type: "mdxJsxAttribute",
						name,
						value: String(value),
					})),
			};
		},
	},
};
