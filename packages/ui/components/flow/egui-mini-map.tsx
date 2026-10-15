interface Bounds {
	x: number;
	y: number;
	width: number;
	height: number;
}

interface MiniMapRectangle extends Bounds {
	color: string;
}

export function sameMiniMapRectangles(
	previous: readonly MiniMapRectangle[],
	next: readonly MiniMapRectangle[],
): boolean {
	return (
		previous.length === next.length &&
		next.every((rect, index) => {
			const old = previous[index];
			return (
				old.x === rect.x &&
				old.y === rect.y &&
				old.width === rect.width &&
				old.height === rect.height &&
				old.color === rect.color
			);
		})
	);
}

export function miniMapProjection(nodes: Bounds | undefined, view: Bounds) {
	const left = Math.min(nodes?.x ?? view.x, view.x);
	const top = Math.min(nodes?.y ?? view.y, view.y);
	const right = Math.max(
		nodes ? nodes.x + nodes.width : view.x + view.width,
		view.x + view.width,
	);
	const bottom = Math.max(
		nodes ? nodes.y + nodes.height : view.y + view.height,
		view.y + view.height,
	);
	const scale = Math.max((right - left) / 200, (bottom - top) / 150, 0.001);
	const offset = 5 * scale;
	return {
		x: left - (scale * 200 - (right - left)) / 2 - offset,
		y: top - (scale * 150 - (bottom - top)) / 2 - offset,
		width: scale * 200 + offset * 2,
		height: scale * 150 + offset * 2,
		scale,
		offset,
	};
}
