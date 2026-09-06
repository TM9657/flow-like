import { expect, test } from "bun:test";
import {
	GRAPH_COMPACT_MIN_STAGE_HEIGHT,
	GRAPH_MIN_STAGE_HEIGHT,
	GRAPH_QUERY_DOCK_DEFAULT_HEIGHT,
	GRAPH_QUERY_DOCK_MIN_HEIGHT,
	clampGraphQueryDockHeight,
	getGraphShellMode,
	getGraphStageMinHeight,
} from "./graph-shell-layout";

test("graph controls respond to their container instead of the browser viewport", () => {
	expect(getGraphShellMode(1200)).toEqual({
		compactToolbar: false,
		overlayInspector: false,
		compactLegend: false,
	});
	expect(getGraphShellMode(1000)).toEqual({
		compactToolbar: true,
		overlayInspector: false,
		compactLegend: false,
	});
	expect(getGraphShellMode(640)).toEqual({
		compactToolbar: true,
		overlayInspector: true,
		compactLegend: true,
	});
});

test("query dock keeps a usable graph stage and remains bounded", () => {
	expect(clampGraphQueryDockHeight(40, 700)).toBe(GRAPH_QUERY_DOCK_MIN_HEIGHT);
	expect(clampGraphQueryDockHeight(900, 700)).toBe(
		700 - GRAPH_MIN_STAGE_HEIGHT,
	);
	expect(getGraphStageMinHeight(390)).toBe(246);
	expect(clampGraphQueryDockHeight(900, 390)).toBe(GRAPH_QUERY_DOCK_MIN_HEIGHT);
	expect(getGraphStageMinHeight(300)).toBe(GRAPH_COMPACT_MIN_STAGE_HEIGHT);
	expect(clampGraphQueryDockHeight(Number.NaN, 700)).toBe(
		GRAPH_QUERY_DOCK_DEFAULT_HEIGHT,
	);
});
