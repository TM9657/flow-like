import type { IBoard } from "../schema/flow/board";

const snapshots = new WeakSet<object>();

/** Sync and media projection already preserve unchanged objects. */
export function markBoardSnapshot(board: IBoard): IBoard {
	snapshots.add(board);
	return board;
}

export function isBoardSnapshot(value: unknown): value is IBoard {
	return typeof value === "object" && value !== null && snapshots.has(value);
}
