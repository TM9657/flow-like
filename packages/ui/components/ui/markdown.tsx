import { transformerColorizedBrackets } from "@shikijs/colorized-brackets";
import rehypeShiki from "@shikijs/rehype";
import {
	transformerNotationDiff,
	transformerNotationErrorLevel,
	transformerNotationFocus,
	transformerNotationHighlight,
	transformerNotationWordHighlight,
	transformerRenderWhitespace,
} from "@shikijs/transformers";
import { toast } from "sonner";
// https://github.com/shikijs/shiki/issues/829
import useMarkdown from "../../hooks/use-markdown";

export function MarkdownComponent({ content }: Readonly<{ content: string }>) {
	const [markdown] = useMarkdown(content, {
		rehypePlugins: [
			[
				rehypeShiki,
				{
					themes: {
						// https://textmate-grammars-themes.netlify.app/?theme=min-dark&grammar=javascript
						light: "everforest-light",
						dark: "min-dark",
					},
					wrap: true,
					transformers: [
						transformerRenderWhitespace(),
						transformerNotationHighlight(),
						transformerNotationWordHighlight(),
						transformerNotationFocus(),
						transformerNotationErrorLevel(),
						transformerNotationDiff(),
						transformerColorizedBrackets(),
						{
							name: "copy-button",
							pre(node: any) {
								const button = {
									type: "element",
									tagName: "button",
									properties: {
										className: "copy-code-button",
										onClick: () => {
											navigator.clipboard.writeText(`${(this as any)?.source}`);
											toast.success("Code copied to clipboard");
										},
										ariaLabel: "Copy code",
									},
									children: [
										{
											type: "text",
											value: "Copy",
										},
									],
								};
								node.children.push(button);
								return node;
							},
						},
					],
				},
			],
		],
	});

	return markdown;
}
