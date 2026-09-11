import { create } from "zustand";
import {
	type IBoardQualityReport,
	type IQualityMark,
	type IQualitySeverity,
	emptyQualityReport,
} from "../lib/board-quality";

const INLINE_KEY = "flow-board-quality-inline";

function readInline(): boolean {
	if (typeof window === "undefined") return true;
	try {
		return window.localStorage.getItem(INLINE_KEY) !== "off";
	} catch {
		return true;
	}
}

function writeInline(inline: boolean): void {
	if (typeof window === "undefined") return;
	try {
		window.localStorage.setItem(INLINE_KEY, inline ? "on" : "off");
	} catch {
		/* private mode */
	}
}

export type IQualitySeverityFilter = Record<IQualitySeverity, boolean>;

interface IBoardQualityState {
	/** boardId → the latest report. */
	reports: Record<string, IBoardQualityReport>;
	/**
	 * boardId → target id → mark. A mark object is reused across reports while
	 * its signature holds, so a node's `marks[board]?.[id]` selector returns the
	 * same reference and the node skips its re-render.
	 */
	marks: Record<string, Record<string, IQualityMark>>;
	/** Whether the canvas shows badges on affected nodes. */
	inline: boolean;
	filter: IQualitySeverityFilter;
	setReport: (boardId: string, report: IBoardQualityReport) => void;
	clear: (boardId: string) => void;
	setInline: (inline: boolean) => void;
	toggleSeverity: (severity: IQualitySeverity) => void;
}

/** Keeps every previous mark whose signature is unchanged; returns the old map if nothing moved. */
export function mergeMarks(
	previous: Record<string, IQualityMark> | undefined,
	next: Record<string, IQualityMark>,
): Record<string, IQualityMark> {
	const merged: Record<string, IQualityMark> = {};
	let changed = false;
	for (const [id, mark] of Object.entries(next)) {
		const old = previous?.[id];
		if (old && old.signature === mark.signature) {
			merged[id] = old;
		} else {
			merged[id] = mark;
			changed = true;
		}
	}
	if (
		!changed &&
		previous &&
		Object.keys(previous).length === Object.keys(merged).length
	) {
		return previous;
	}
	return merged;
}

export const useBoardQualityStore = create<IBoardQualityState>((set) => ({
	reports: {},
	marks: {},
	inline: readInline(),
	filter: { error: true, warning: true, info: true },
	setReport: (boardId, report) =>
		set((state) => {
			const marks = mergeMarks(state.marks[boardId], report.marks);
			return {
				reports: { ...state.reports, [boardId]: report },
				marks:
					marks === state.marks[boardId]
						? state.marks
						: { ...state.marks, [boardId]: marks },
			};
		}),
	clear: (boardId) =>
		set((state) => {
			if (!state.reports[boardId] && !state.marks[boardId]) return state;
			const reports = { ...state.reports };
			const marks = { ...state.marks };
			delete reports[boardId];
			delete marks[boardId];
			return { reports, marks };
		}),
	setInline: (inline) => {
		writeInline(inline);
		set({ inline });
	},
	toggleSeverity: (severity) =>
		set((state) => ({
			filter: { ...state.filter, [severity]: !state.filter[severity] },
		})),
}));

const EMPTY_REPORT = emptyQualityReport();

export function selectQualityReport(
	boardId: string,
): (state: IBoardQualityState) => IBoardQualityReport {
	return (state) => state.reports[boardId] ?? EMPTY_REPORT;
}

/** `undefined` while inline marks are off, so a node subscribes to nothing that moves. */
export function selectQualityMark(
	boardId: string,
	targetId: string,
): (state: IBoardQualityState) => IQualityMark | undefined {
	return (state) =>
		state.inline ? state.marks[boardId]?.[targetId] : undefined;
}
