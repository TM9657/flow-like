/** Matches the node title's existing character-based fitting in both renderers. */
export function autoResizeTextScale(
	text: string,
	maxChars = 12,
	minFontSize = 0.6,
): number {
	return text.length <= maxChars
		? 1
		: Math.max((maxChars / text.length) ** 1.3, minFontSize);
}
