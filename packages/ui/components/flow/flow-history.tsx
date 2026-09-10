import { useTranslation } from "@flow-like/locales";
import { type UseQueryResult, useQueryClient } from "@tanstack/react-query";
import { HistoryIcon, Redo2Icon, Undo2Icon, XIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useSyncExternalStore } from "react";
import { injectData } from "../../hooks/use-invoke";
import { getErrorMessage } from "../../lib/error-message";
import {
	type HistorySummary,
	decodeEntry,
	isTransientReplayError,
} from "../../lib/flow-history";
import { createHistoryPersistence } from "../../lib/flow-history-db";
import {
	BoardHistoryRegistry,
	type HistoryDirection,
	type HistoryRecordMode,
	installRemoteBoardAppliedListener,
} from "../../lib/flow-history-store";
import { toastError, toastSuccess, toastWarning } from "../../lib/messages";
import type { IGenericCommand } from "../../lib/schema";
import type { IBoard } from "../../lib/schema/flow/board";
import { useBackendStore } from "../../state/backend-state";

export const historyRegistry = new BoardHistoryRegistry(
	createHistoryPersistence(),
);

// A server-side board reset must invalidate history even when the board is not mounted
// anywhere: the entries were recorded against a board that no longer exists.
installRemoteBoardAppliedListener(historyRegistry);

/**
 * Recording side of a board's history. The callbacks are stable per board and resolve the
 * history lazily, so a component may keep them for as long as it likes.
 */
export const useUndoRedo = (appId: string, boardId: string) =>
	useMemo(
		() => ({
			pushCommand: async (command: IGenericCommand): Promise<void> => {
				await historyRegistry.get(appId, boardId).record([command]);
			},
			pushCommands: async (commands: IGenericCommand[]): Promise<void> => {
				await historyRegistry.get(appId, boardId).record(commands);
			},
			pushCommandsOnce: async (
				commands: IGenericCommand[],
				deliveryId: string,
				historyMode: HistoryRecordMode = "append",
			): Promise<void> => {
				await historyRegistry
					.get(appId, boardId)
					.recordOnce(commands, deliveryId, historyMode);
			},
			/**
			 * Serialize a board mutation against undo/redo. An edit committed inside it is recorded
			 * before any later undo runs, so undo always targets the last edit the user saw land.
			 */
			withHistoryLock<T>(operation: () => Promise<T>): Promise<T> {
				return historyRegistry.get(appId, boardId).exclusive(operation);
			},
		}),
		[appId, boardId],
	);

/** Keeps the board's history resident (and hydrated) for as long as the caller is mounted. */
export function useRetainBoardHistory(appId: string, boardId: string): void {
	useEffect(() => historyRegistry.retain(appId, boardId), [appId, boardId]);
}

/** Undo/redo depth for toolbars and command palettes; re-renders only when it changes. */
export function useBoardHistory(
	appId: string,
	boardId: string,
): HistorySummary {
	const subscribe = useCallback(
		(listener: () => void) =>
			historyRegistry.get(appId, boardId).subscribe(listener),
		[appId, boardId],
	);
	const getSnapshot = useCallback(
		() => historyRegistry.get(appId, boardId).getSummary(),
		[appId, boardId],
	);
	return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

interface HistoryNavigationProps {
	appId: string;
	boardId: string;
	board: UseQueryResult<IBoard>;
	version: [number, number, number] | undefined;
	/**
	 * Advisory hook fired with the batch about to be replayed BEFORE it executes — the board
	 * surfaces a toast when the batch touches statements a peer is editing. Never blocks.
	 */
	onHistoryBatch?: (commands: IGenericCommand[]) => void;
}

/**
 * Replay side of a board's history: undo and redo as single operations that step the cursor,
 * replay on the backend, and bring the board the backend produced into the query cache.
 *
 * Both run under the board's history lock. A backend refusal moves the cursor back; a repeated
 * refusal of the same entry drops it, because the board no longer holds what it describes.
 */
export function useHistoryNavigation({
	appId,
	boardId,
	board,
	version,
	onHistoryBatch,
}: HistoryNavigationProps) {
	const { t } = useTranslation("flow");
	const queryClient = useQueryClient();
	const refetchBoard = board.refetch;

	const replay = useCallback(
		async (direction: HistoryDirection): Promise<boolean> => {
			if (typeof version !== "undefined") {
				toastError(
					t("cannotChangeOldVersion", "Cannot change old version"),
					<XIcon />,
				);
				return false;
			}
			const backend = useBackendStore.getState().backend;
			if (!backend) return false;
			const history = historyRegistry.get(appId, boardId);
			const label =
				direction === "undo" ? t("undo", "Undo") : t("redo", "Redo");
			const icon =
				direction === "undo" ? (
					<Undo2Icon className="w-4 h-4" />
				) : (
					<Redo2Icon className="w-4 h-4" />
				);

			return history.exclusive(async () => {
				const entry = await history.take(direction);
				if (!entry) return false;

				let commands: IGenericCommand[];
				try {
					commands = decodeEntry(entry);
				} catch (error) {
					console.error(
						`[flow-history] Corrupt ${direction} entry dropped:`,
						error,
					);
					history.discard(entry);
					toastWarning(
						t("historyEntryUnreadableSkipped", {
							defaultValue:
								"{{action}}: one recorded change was unreadable and was skipped",
							action: label,
						}),
						<HistoryIcon className="w-4 h-4" />,
					);
					return false;
				}

				onHistoryBatch?.(commands);
				let resultingBoard: IBoard | undefined;
				const options = {
					onBoard: (next: IBoard) => {
						resultingBoard = next;
					},
				};
				try {
					if (direction === "undo") {
						await backend.boardState.undoBoard(
							appId,
							boardId,
							commands,
							options,
						);
					} else {
						await backend.boardState.redoBoard(
							appId,
							boardId,
							commands,
							options,
						);
					}
				} catch (error) {
					const dropped = history.fail(
						direction,
						entry,
						isTransientReplayError(error),
					);
					const message = getErrorMessage(error, "Unknown error");
					console.error(`[flow-history] ${label} failed:`, error);
					toastError(
						dropped
							? t("historyEntrySkippedAfterRepeatedFailure", {
									defaultValue:
										"{{action}} failed again for the same change, so it was skipped: {{message}}",
									action: label,
									message,
								})
							: t("historyActionFailed", {
									defaultValue: "{{action}} failed: {{message}}",
									action: label,
									message,
								}),
						<XIcon />,
					);
					await refetchBoard().catch(() => undefined);
					return false;
				}

				history.confirm();
				if (resultingBoard) {
					injectData(
						queryClient,
						backend.boardState.getBoard,
						[appId, boardId],
						resultingBoard,
					);
				} else {
					await refetchBoard().catch((error) => {
						console.warn(
							`[flow-history] Board refetch after ${direction} failed:`,
							error,
						);
					});
				}
				toastSuccess(label, icon);
				return true;
			});
		},
		[appId, boardId, refetchBoard, version, onHistoryBatch, queryClient, t],
	);

	const undo = useCallback(() => replay("undo"), [replay]);
	const redo = useCallback(() => replay("redo"), [replay]);
	return { undo, redo };
}
