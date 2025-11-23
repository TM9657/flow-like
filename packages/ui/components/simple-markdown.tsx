"use client";

import { memo } from "react";

interface SimpleMarkdownProps {
	children: string;
	className?: string;
}

export const SimpleMarkdown = memo(function SimpleMarkdown({
	children,
	className,
}: SimpleMarkdownProps) {
	// Convert escaped newlines to actual newlines
	const formatted = children
		.replace(/\\n/g, "\n")
		.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>")
		.replace(/\*(.*?)\*/g, "<em>$1</em>")
		.replace(/`([^`]+)`/g, "<code>$1</code>");

	return (
		<div
			className={className}
			style={{ whiteSpace: "pre-wrap" }}
			dangerouslySetInnerHTML={{ __html: formatted }}
		/>
	);
});
