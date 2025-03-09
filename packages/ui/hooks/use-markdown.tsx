import { useCallback, useEffect, useState } from "react";
import * as jsxRuntime from "react/jsx-runtime";
import rehypeReact, { type Options as RehypeReactOptions } from "rehype-react";
import remarkParse, { type Options as RemarkParseOptions } from "remark-parse";
import remarkRehype, {
	type Options as RemarkRehypeOptions,
} from "remark-rehype";
import { type PluggableList, unified } from "unified";

export interface UseRemarkOptions {
	remarkParseOptions?: RemarkParseOptions;
	remarkPlugins?: PluggableList;
	remarkRehypeOptions?: RemarkRehypeOptions;
	rehypePlugins?: PluggableList;
	rehypeReactOptions?: Pick<RehypeReactOptions, "components">;
	onError?: (err: Error) => void;
}

export default function useMarkdown(
	markdown: string,
	{
		remarkParseOptions,
		remarkPlugins = [],
		remarkRehypeOptions,
		rehypePlugins = [],
		rehypeReactOptions,
		onError = () => {},
	}: UseRemarkOptions = {},
): [React.ReactElement | null] {
	const [content, setContent] = useState<React.ReactElement | null>(null);

	const setMarkdown = useCallback((source: string) => {
		unified()
			.use(remarkParse, remarkParseOptions)
			.use(remarkPlugins)
			.use(remarkRehype, remarkRehypeOptions)
			.use(rehypePlugins)
			.use(rehypeReact, {
				...rehypeReactOptions,
				Fragment: jsxRuntime.Fragment,
				jsx: jsxRuntime.jsx as any,
				jsxs: jsxRuntime.jsxs as any,
			} satisfies RehypeReactOptions)
			.process(source)
			.then((vfile: { result: React.ReactElement }) => setContent(vfile.result))
			.catch(onError);
	}, []);

	useEffect(() => {
		setMarkdown(markdown);
	}, [markdown]);

	return [content];
}
