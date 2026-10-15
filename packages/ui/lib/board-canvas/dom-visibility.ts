import type { Edge, InternalNode, Node } from "@xyflow/react";

export const WARM_EDITOR_LIMIT = 256;

/** Small editor populations stay mounted so camera changes do not rebuild them. */
export function warmEditorIds(nodes: readonly Node[]): ReadonlySet<string> {
	const ordinary = nodes.filter(
		(node) => node.type === "node" || node.type === "flowNode",
	);
	return new Set(
		ordinary.length <= WARM_EDITOR_LIMIT ? ordinary.map((node) => node.id) : [],
	);
}

export interface DomVisibilityOptions {
	nodes: readonly Node[];
	edges: readonly Edge[];
	nodeLookup: ReadonlyMap<string, InternalNode>;
	transform: readonly [number, number, number];
	width: number;
	height: number;
	retained?: ReadonlySet<string>;
}

/** Keep a screen-space margin so rich editors mount before entering the camera. */
export function visibleDomIds({
	nodes,
	edges,
	nodeLookup,
	transform: [x, y, zoom],
	width,
	height,
	retained,
}: DomVisibilityOptions): { nodes: string[]; edges: string[] } {
	if (width <= 0 || height <= 0 || zoom <= 0)
		return {
			nodes: nodes.map((node) => node.id),
			edges: edges.map((edge) => edge.id),
		};
	const margin = 200;
	const view = {
		left: (-x - margin) / zoom,
		top: (-y - margin) / zoom,
		right: (width - x + margin) / zoom,
		bottom: (height - y + margin) / zoom,
	};
	const bounds = (id: string) => {
		const node = nodeLookup.get(id);
		const width = node?.measured.width ?? node?.width;
		const height = node?.measured.height ?? node?.height;
		if (!node || width === undefined || height === undefined) return;
		return {
			left: node.internals.positionAbsolute.x,
			top: node.internals.positionAbsolute.y,
			right: node.internals.positionAbsolute.x + width,
			bottom: node.internals.positionAbsolute.y + height,
		};
	};
	const intersects = (rect: typeof view) =>
		rect.left <= view.right &&
		rect.right >= view.left &&
		rect.top <= view.bottom &&
		rect.bottom >= view.top;
	return {
		nodes: nodes
			.filter((node) => {
				if (
					node.selected ||
					node.dragging ||
					node.data.__flowNativeHovered ||
					retained?.has(node.id)
				)
					return true;
				const rect = bounds(node.id);
				return !rect || intersects(rect);
			})
			.map((node) => node.id),
		edges: edges
			.filter((edge) => {
				if (edge.selected || retained?.has(edge.id)) return true;
				const source = bounds(edge.source);
				const target = bounds(edge.target);
				if (!source || !target) return true;
				// Custom paths can bend beyond either endpoint. A generous world-space
				// margin retains those curves while culling distant SVG-only regions.
				const padding = Math.max(
					300,
					Math.abs(source.left - target.left) / 2,
					Math.abs(source.top - target.top) / 2,
				);
				return intersects({
					left: Math.min(source.left, target.left) - padding,
					top: Math.min(source.top, target.top) - padding,
					right: Math.max(source.right, target.right) + padding,
					bottom: Math.max(source.bottom, target.bottom) + padding,
				});
			})
			.map((edge) => edge.id),
	};
}

export function equalVisibleIds(
	a: ReturnType<typeof visibleDomIds>,
	b: ReturnType<typeof visibleDomIds>,
) {
	return (
		a.nodes.length === b.nodes.length &&
		a.edges.length === b.edges.length &&
		a.nodes.every((id, index) => id === b.nodes[index]) &&
		a.edges.every((id, index) => id === b.edges[index])
	);
}
