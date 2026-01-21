"use client";

import { useMemo } from "react";
import * as prod from "react/jsx-runtime";
import rehypeReact from "rehype-react";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, MarkdownComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

const production = { Fragment: prod.Fragment, jsx: prod.jsx, jsxs: prod.jsxs };

export function A2UIMarkdown({
	component,
	style,
}: ComponentProps<MarkdownComponent>) {
	const content = useResolved<string>(component.content);

	const rendered = useMemo(() => {
		if (!content) return null;
		const result = unified()
			.use(remarkParse)
			.use(remarkRehype)
			.use(rehypeReact, production)
			.processSync(content);
		return result.result;
	}, [content]);

	return (
		<div
			className={cn(
				"prose prose-sm dark:prose-invert max-w-none",
				resolveStyle(style),
			)}
			style={resolveInlineStyle(style)}
		>
			{rendered}
		</div>
	);
}
