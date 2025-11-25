"use client";

import type { TLinkElement } from "platejs";
import type { PlateElementProps } from "platejs/react";

import { getLinkAttributes } from "@platejs/link";
import { PlateElement } from "platejs/react";

export function LinkElement(props: PlateElementProps<TLinkElement>) {
	const url = props.element.url;

	// Check if this is a focus node link
	if (typeof url === "string" && url.startsWith("focus://")) {
		const nodeId = url.replace("focus://", "");
		return (
			<span
				{...props.attributes}
				data-focus-node-id={nodeId}
				className="inline-flex items-center gap-1 px-1.5 py-0.5 mx-0.5 text-xs font-medium text-primary bg-primary/10 hover:bg-primary/20 rounded-md border border-primary/20 hover:border-primary/40 transition-all cursor-pointer"
				contentEditable={false}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					className="shrink-0"
				>
					<path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
				</svg>
				{props.children}
			</span>
		);
	}

	return (
		<PlateElement
			{...props}
			as="a"
			className="font-medium text-primary underline decoration-primary underline-offset-4"
			attributes={{
				...props.attributes,
				...getLinkAttributes(props.editor, props.element),
				onMouseOver: (e) => {
					e.stopPropagation();
				},
			}}
		>
			{props.children}
		</PlateElement>
	);
}
