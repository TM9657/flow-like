"use client";

import type { SlateElementProps } from "platejs";

import { SlateElement } from "platejs";

export interface TFocusNodeElement {
	type: "focus_node";
	nodeId: string;
	nodeName: string;
	children: [{ text: "" }];
	[key: string]: unknown;
}

export function FocusNodeElementStatic(
	props: SlateElementProps<TFocusNodeElement>,
) {
	const { nodeId, nodeName } = props.element;

	return (
		<SlateElement
			{...props}
			as="span"
			className="
				inline-flex items-center gap-1.5
				px-1 pr-2 py-0.5
				-my-0.5 mx-0.5
				align-baseline
				text-xs font-semibold tracking-tight
				text-violet-700 dark:text-violet-300
				bg-gradient-to-r from-violet-100 via-purple-50 to-fuchsia-100
				dark:from-violet-950/60 dark:via-purple-950/40 dark:to-fuchsia-950/60
				rounded-full
				border border-violet-200/60 dark:border-violet-700/40
				shadow-sm shadow-violet-200/50 dark:shadow-violet-900/30
				hover:shadow-md hover:shadow-violet-300/50 dark:hover:shadow-violet-800/40
				hover:border-violet-300 dark:hover:border-violet-600
				hover:from-violet-200 hover:via-purple-100 hover:to-fuchsia-200
				dark:hover:from-violet-900/70 dark:hover:via-purple-900/50 dark:hover:to-fuchsia-900/70
				hover:scale-[1.02]
				active:scale-[0.98]
				transition-all duration-200 ease-out
				cursor-pointer
				select-none
			"
			attributes={{
				...props.attributes,
				"data-focus-node-id": nodeId,
			}}
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="11"
				height="11"
				viewBox="0 0 24 24"
				fill="currentColor"
				className="shrink-0 opacity-80"
			>
				<path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
			</svg>
			<span contentEditable={false} className="leading-none">
				{nodeName}
			</span>
			{props.children}
		</SlateElement>
	);
}
