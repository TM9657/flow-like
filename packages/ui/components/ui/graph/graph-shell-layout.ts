export const GRAPH_MIN_STAGE_HEIGHT = 280;
export const GRAPH_COMPACT_MIN_STAGE_HEIGHT = 220;
/** Fits the title, natural-language row, and editable generated-query row. */
export const GRAPH_QUERY_DOCK_MIN_HEIGHT = 168;
export const GRAPH_QUERY_DOCK_DEFAULT_HEIGHT = 240;

const COMPACT_TOOLBAR_WIDTH = 1100;
const OVERLAY_INSPECTOR_WIDTH = 960;
const COMPACT_LEGEND_WIDTH = 720;

export interface GraphShellMode {
	compactToolbar: boolean;
	overlayInspector: boolean;
	compactLegend: boolean;
}

/**
 * The graph often lives beside application navigation, so viewport media
 * queries do not describe the space its controls can actually use.
 */
export function getGraphShellMode(containerWidth: number): GraphShellMode {
	if (!Number.isFinite(containerWidth) || containerWidth <= 0) {
		return {
			compactToolbar: false,
			overlayInspector: false,
			compactLegend: false,
		};
	}

	return {
		compactToolbar: containerWidth < COMPACT_TOOLBAR_WIDTH,
		overlayInspector: containerWidth < OVERLAY_INSPECTOR_WIDTH,
		compactLegend: containerWidth < COMPACT_LEGEND_WIDTH,
	};
}

export function clampGraphQueryDockHeight(
	requestedHeight: number,
	workspaceHeight: number,
): number {
	const requested = Number.isFinite(requestedHeight)
		? requestedHeight
		: GRAPH_QUERY_DOCK_DEFAULT_HEIGHT;
	if (!Number.isFinite(workspaceHeight) || workspaceHeight <= 0) {
		return Math.max(requested, GRAPH_QUERY_DOCK_MIN_HEIGHT);
	}

	const maximum = Math.max(
		0,
		workspaceHeight - getGraphStageMinHeight(workspaceHeight),
	);
	const minimum = Math.min(GRAPH_QUERY_DOCK_MIN_HEIGHT, maximum);

	return Math.min(Math.max(requested, minimum), maximum);
}

/** Keeps the ideal stage size until a short container must share with the dock. */
export function getGraphStageMinHeight(workspaceHeight: number): number {
	if (!Number.isFinite(workspaceHeight) || workspaceHeight <= 0) {
		return GRAPH_MIN_STAGE_HEIGHT;
	}

	return Math.min(
		GRAPH_MIN_STAGE_HEIGHT,
		Math.max(
			GRAPH_COMPACT_MIN_STAGE_HEIGHT,
			workspaceHeight - GRAPH_QUERY_DOCK_MIN_HEIGHT,
		),
	);
}
