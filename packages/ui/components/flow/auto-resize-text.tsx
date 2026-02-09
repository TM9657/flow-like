import type React from "react";
import { useMemo } from "react";

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

	const fontSize = useMemo(() => {
		if (content.length <= maxChars) return undefined;

		// More aggressive scaling: use a power curve
		const ratio = maxChars / content.length;
		// Power of 1.3 makes it drop faster than linear
		return Math.max(Math.pow(ratio, 1.3), minFontSize);
	}, [content, maxChars, minFontSize]);

	return (
		<span
			className={`truncate ${className}`}
			style={{
				fontSize: fontSize ? `${fontSize}em` : undefined,
			}}
			{...props}
		>
			{content}
		</span>
	);
}
