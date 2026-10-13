"use client";
import {
	type DeserializeMdOptions,
	MarkdownPlugin,
	remarkMdx,
	remarkMention,
} from "@platejs/markdown";
import { KEYS, TextApi, type Value } from "platejs";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

import { editorialMediaMarkdown } from "./editorial-media-markdown";
import { deserializeMdKeepingCode } from "./markdown-code-guard";
import { withoutUnsafeUrls } from "./safe-url-kit";
import { tableMarkdown } from "./table-markdown";

export const MarkdownKit = [
	MarkdownPlugin.configure({
		options: {
			disallowedNodes: [KEYS.suggestion],
			plainMarks: [KEYS.comment],
			rules: { ...editorialMediaMarkdown, ...tableMarkdown },
			remarkPlugins: [
				[
					remarkMath,
					{ singleDollarTextMath: false },
				] as unknown as typeof remarkMath,
				remarkGfm,
				remarkMdx,
				remarkMention,
			],
		},
	}).extendApi(({ editor }) => ({
		/** Links and media with script or non-image `data:` URLs never leave the parser. */
		deserialize: (
			data: string,
			options?: Omit<DeserializeMdOptions, "editor">,
		): Value =>
			withoutUnsafeUrls(
				editor,
				deserializeMdKeepingCode(editor, data, options),
			).map((node) =>
				TextApi.isText(node)
					? { type: editor.getType(KEYS.p), children: [node] }
					: node,
			),
	})),
];
