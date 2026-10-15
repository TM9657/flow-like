import type React from "react";
import { useMemo } from "react";
import { autoResizeTextScale } from "./auto-resize-text-size";

interface AutoResizeTextProps extends React.HTMLAttributes<HTMLElement> {
	text: string | undefined | null;
	maxChars?: number;
	minFontSize?: number; // in em, e.g. 0.6
}

export function AutoResizeText({
	text,
	className,
	maxChars = 12,
	minFontSize = 0.6,
	...props
}: AutoResizeTextProps) {
	const content = text || "";

	const fontSize = useMemo(
		() => autoResizeTextScale(content, maxChars, minFontSize),
		[content, maxChars, minFontSize],
	);

	return (
		<span
			className={`truncate ${className}`}
			style={{
				fontSize: fontSize === 1 ? undefined : `${fontSize}em`,
			}}
			{...props}
		>
			{content}
		</span>
	);
}
