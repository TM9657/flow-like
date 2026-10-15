export function canvasGridMultiplier(zoom: number, gap = 12): number {
	if (!(zoom > 0) || !Number.isFinite(zoom)) return 1;
	return 2 ** Math.max(0, Math.ceil(Math.log2(8 / (gap * zoom))));
}

export function canvasGridLayout(multiplier: number, gap = 12) {
	return {
		gap: gap * multiplier,
		// React Flow's default half-gap offset moves dots when the gap changes.
		dotOffset: multiplier > 1 ? gap / 2 : undefined,
	};
}
