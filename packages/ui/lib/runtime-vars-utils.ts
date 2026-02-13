import type { IBoard, IVariable } from "../lib/schema/flow/board";

/**
 * Get all runtime-configured variables from a board (including secrets)
 */
export function getRuntimeConfiguredVariables(board: IBoard): IVariable[] {
	return Object.values(board.variables).filter(
		(v) => v.runtime_configured || v.secret,
	);
}

/**
 * Get IDs of all runtime-configured variables from a board
 */
export function getRuntimeConfiguredVariableIds(board: IBoard): string[] {
	return getRuntimeConfiguredVariables(board).map((v) => v.id);
}

/**
 * Check if a board has any nodes that require offline-only execution
 */
export function hasOfflineOnlyNodes(board: IBoard): boolean {
	return Object.values(board.nodes).some((node) => node.only_offline);
}

/**
 * Get all nodes that require offline-only execution
 */
export function getOfflineOnlyNodes(board: IBoard) {
	return Object.values(board.nodes).filter((node) => node.only_offline);
}
