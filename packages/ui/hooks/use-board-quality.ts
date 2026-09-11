"use client";

import { useEffect } from "react";
import { analyzeBoardQuality } from "../lib/board-quality";
import type { IBoard } from "../lib/schema/flow/board";
import { useBoardQualityStore } from "../state/board-quality-state";

/** Long enough to coalesce a drag or a burst of commands, short enough to feel live. */
const DEBOUNCE_MS = 400;

type IdleWindow = Window & {
	requestIdleCallback?: (
		callback: () => void,
		options?: { timeout: number },
	) => number;
	cancelIdleCallback?: (handle: number) => void;
};

/**
 * Re-lints the board after every settled edit and publishes the report to the
 * quality store. Runs whether or not the panel is open: the rail badge and the
 * inline node marks read from the same report.
 */
export function useBoardQualityAnalysis(
	boardId: string,
	board: IBoard | undefined,
): void {
	useEffect(() => {
		if (!board) return;
		let cancelled = false;
		let idleHandle: number | undefined;
		const idle = window as IdleWindow;

		const run = () => {
			if (cancelled) return;
			useBoardQualityStore
				.getState()
				.setReport(boardId, analyzeBoardQuality(board));
		};
		const timer = window.setTimeout(() => {
			if (cancelled) return;
			if (typeof idle.requestIdleCallback === "function") {
				idleHandle = idle.requestIdleCallback(run, { timeout: 1500 });
			} else {
				run();
			}
		}, DEBOUNCE_MS);

		return () => {
			cancelled = true;
			window.clearTimeout(timer);
			if (idleHandle !== undefined) idle.cancelIdleCallback?.(idleHandle);
		};
	}, [board, boardId]);

	useEffect(
		() => () => useBoardQualityStore.getState().clear(boardId),
		[boardId],
	);
}
