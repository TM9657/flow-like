"use client";

import { remarkMdx, remarkMention } from "@platejs/markdown";
import { PlateStatic, type Value, createSlateEditor } from "platejs";
import { Plate, usePlateEditor } from "platejs/react";
import { memo, useMemo } from "react";
import remarkBreaks from "remark-breaks";
import remarkEmoji from "remark-emoji";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import { BaseEditorKit } from "../editor/editor-base-kit";
import { EditorKit } from "../editor/editor-kit";
import { Editor, EditorContainer } from "../editor/ui/editor";

/**
 * A prefix to identify content that is serialized as Plate's native JSON.
 * This allows switching from initial Markdown to JSON after the first edit.
 */
const PLATE_JSON_PREFIX = "plate_json::";

/**
 * Splits a Markdown string into top-level blocks while preserving the integrity of fenced code blocks.
 * This is crucial for fallback rendering of broken or invalid Markdown.
 * @param markdown The raw markdown string.
 * @returns An array of markdown block strings.
 */
const splitMarkdownPreservingCodeBlocks = (markdown: string): string[] => {
	const blocks: string[] = [];
	const codeBlockRegex = /(^```[\s\S]*?^```$)|(^~~~[\s\S]*?^~~~$)/gm;
	let lastIndex = 0;
	let match;

	while ((match = codeBlockRegex.exec(markdown)) !== null) {
		const precedingText = markdown.substring(lastIndex, match.index);
		if (precedingText.trim()) {
			blocks.push(...precedingText.trim().split(/\n{2,}/));
		}
		blocks.push(match[0]);
		lastIndex = codeBlockRegex.lastIndex;
	}

	const remainingText = markdown.substring(lastIndex);
	if (remainingText.trim()) {
		blocks.push(...remainingText.trim().split(/\n{2,}/));
	}

	return blocks.filter(Boolean);
};

/**
 * Safely deserializes content into Plate editor nodes.
 * It handles prefixed native Plate JSON, Markdown, and plain text, with fallbacks.
 */
export const safeDeserialize = (
	editor: any,
	data: string,
	isMarkdown: boolean,
	remarkPlugins: any[],
): Value => {
	// 1. Check for the native JSON prefix first.
	if (data.startsWith(PLATE_JSON_PREFIX)) {
		try {
			const jsonString = data.substring(PLATE_JSON_PREFIX.length);
			const nodes = JSON.parse(jsonString);
			if (Array.isArray(nodes) && nodes.length > 0) {
				return nodes;
			}
		} catch (error) {
			console.error(
				"Failed to parse prefixed Plate JSON, falling back.",
				error,
			);
			return [{ type: "p", children: [{ text: data }] }];
		}
	}

	// 2. Handle initial content that is not markdown (e.g., plain text or legacy JSON).
	if (!isMarkdown) {
		try {
			// Assuming editor.api.deserialize is a custom function, potentially JSON.parse
			const nodes = editor.api.deserialize(data);
			if (nodes.length > 0) return nodes;
			return [{ type: "p", children: [{ text: data }] }];
		} catch {
			return [{ type: "p", children: [{ text: data }] }];
		}
	}

	// 3. Handle initial markdown content.
	try {
		const nodes = editor.api.markdown.deserialize(data, { remarkPlugins });
		if (nodes.length > 0) return nodes;
		return [{ type: "p", children: [{ text: "" }] }];
	} catch (error) {
		console.error(
			"Markdown deserialization failed, attempting fallback:",
			error,
		);

		// 4. Fallback for broken markdown: split into blocks and deserialize individually.
		const blocks = splitMarkdownPreservingCodeBlocks(data);
		const nodes = blocks.flatMap((block) => {
			try {
				return editor.api.markdown.deserialize(block, { remarkPlugins });
			} catch {
				return { type: "p", children: [{ text: block }] };
			}
		});

		if (nodes.length > 0) return nodes;
		return [{ type: "p", children: [{ text: data }] }];
	}
};

function TextEditorInner({
	initialContent,
	onChange,
	isMarkdown,
}: Readonly<{
	initialContent: string;
	onChange: (content: string) => void;
	isMarkdown?: boolean;
}>) {
	const remarkPlugins = useMemo(
		() => [
			remarkMath,
			remarkGfm,
			remarkBreaks,
			remarkMdx,
			remarkMention,
			remarkEmoji as any,
		],
		[],
	);

	const editor = usePlateEditor(
		{
			id: "rendered-editor",
			plugins: EditorKit,
			value: (self) =>
				safeDeserialize(
					self,
					initialContent,
					isMarkdown ?? false,
					remarkPlugins,
				),
		},
		[initialContent, isMarkdown, remarkPlugins],
	);

	return (
		<Plate
			editor={editor}
			onChange={({ editor }) => {
				// Get the editor's content directly from the `editor.children` property.
				const serializedNodes = editor.children;
				const newContent = `${PLATE_JSON_PREFIX}${JSON.stringify(
					serializedNodes,
				)}`;

				if (newContent === initialContent) {
					return;
				}
				onChange(newContent);
			}}
		>
			<EditorContainer>
				<Editor variant="none" className="px-4 py-2" />
			</EditorContainer>
		</Plate>
	);
}

function TextEditorStatic({
	initialContent,
	isMarkdown,
	minimal = false,
}: Readonly<{
	initialContent: string;
	isMarkdown?: boolean;
	minimal?: boolean;
}>) {
	const remarkPlugins = useMemo(
		() =>
			minimal
				? [remarkGfm, remarkBreaks]
				: [
						remarkMath,
						remarkGfm,
						remarkBreaks,
						remarkMdx,
						remarkMention,
						remarkEmoji as any,
					],
		[minimal],
	);

	// Use minimal plugin set for better performance in read-only contexts
	const plugins = useMemo(
		() =>
			minimal
				? [
						...BaseEditorKit.filter((plugin) => {
							// Only include essential plugins for markdown rendering
							const pluginId =
								(plugin as any).key || (plugin as any).name || "";
							return (
								pluginId.includes("paragraph") ||
								pluginId.includes("heading") ||
								pluginId.includes("code") ||
								pluginId.includes("list") ||
								pluginId.includes("link") ||
								pluginId.includes("bold") ||
								pluginId.includes("italic") ||
								pluginId.includes("blockquote") ||
								pluginId.includes("markdown")
							);
						}),
					]
				: BaseEditorKit,
		[minimal],
	);

	// The value is memoized to avoid re-creating the editor on every render.
	const value = useMemo(() => {
		// For large content, truncate to prevent performance issues
		const MAX_LENGTH = 50000; // ~50KB
		const contentToRender =
			initialContent.length > MAX_LENGTH
				? initialContent.slice(0, MAX_LENGTH) +
					"\\n\\n... (content truncated for performance)"
				: initialContent;

		const tempEditor = createSlateEditor({ plugins });
		return safeDeserialize(
			tempEditor,
			contentToRender,
			isMarkdown ?? false,
			remarkPlugins,
		);
	}, [initialContent, isMarkdown, remarkPlugins, plugins]);

	const editor = useMemo(
		() =>
			createSlateEditor({
				id: "static-rendered-editor",
				plugins,
				value,
			}),
		[plugins, value],
	);

	return <PlateStatic editor={editor} className="py-0" />;
}

type TextEditorProps = {
	initialContent: string;
	onChange?: (content: string) => void;
	isMarkdown?: boolean;
	editable?: boolean;
	minimal?: boolean;
};

export const TextEditor = memo(function TextEditor({
	initialContent,
	onChange,
	isMarkdown,
	editable = false,
	minimal = false,
}: Readonly<TextEditorProps>) {
	if (editable && onChange) {
		return (
			<TextEditorInner
				initialContent={initialContent}
				onChange={(content: string) => {
					onChange(content);
				}}
				isMarkdown={isMarkdown}
			/>
		);
	}
	return (
		<TextEditorStatic
			initialContent={initialContent}
			isMarkdown={isMarkdown}
			minimal={minimal}
		/>
	);
});
