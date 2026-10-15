export interface PinchCamera {
	x: number;
	y: number;
	zoom: number;
}
export interface PinchPoint {
	x: number;
	y: number;
}

/** Keep the initial world centroid beneath the moving fingers, even at zoom limits. */
export function pinchViewport(
	initialZoom: number,
	initialDistance: number,
	anchor: PinchPoint,
	center: PinchPoint,
	distance: number,
	minZoom: number,
	maxZoom: number,
): PinchCamera {
	const zoom = Math.max(
		minZoom,
		Math.min(maxZoom, (initialZoom * distance) / Math.max(1, initialDistance)),
	);
	return { x: center.x - anchor.x * zoom, y: center.y - anchor.y * zoom, zoom };
}
