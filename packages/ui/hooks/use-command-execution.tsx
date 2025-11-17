import type { UseQueryResult } from "@tanstack/react-query";
import { XIcon } from "lucide-react";
import { useCallback, useRef } from "react";
import { toastError } from "../lib/messages";
import type { IGenericCommand } from "../lib/schema";
import type { IBoard } from "../lib/schema/flow/board";
import { useBackendStore } from "../state/backend-state";

interface UseCommandExecutionProps {
	appId: string;
	boardId: string;
	board: UseQueryResult<IBoard>;
	version: [number, number, number] | undefined;
	pushCommand: (command: any, append?: boolean) => Promise<void>;
	pushCommands: (commands: any[]) => Promise<void>;
}

export function useCommandExecution({
	appId,
	boardId,
	board,
	version,
	pushCommand,
	pushCommands,
}: UseCommandExecutionProps) {
	const awarenessRef = useRef<any | undefined>(undefined);

	const executeCommand = useCallback(
		async (command: IGenericCommand, append = false): Promise<any> => {
			const backend = useBackendStore.getState().backend;
			if (!backend) return;
			if (typeof version !== "undefined") {
				toastError("Cannot change old version", <XIcon />);
				return;
			}
			const result = await backend.boardState.executeCommand(
				appId,
				boardId,
				command,
			);
			await pushCommand(result, append);
			await board.refetch();

			if (awarenessRef.current) {
				awarenessRef.current.setLocalStateField("boardUpdate", Date.now());
			}

			return result;
		},
		[board.refetch, appId, boardId, pushCommand, version],
	);

	const executeCommands = useCallback(
		async (commands: IGenericCommand[]) => {
			const backend = useBackendStore.getState().backend;
			if (!backend) return;
			if (typeof version !== "undefined") {
				toastError("Cannot change old version", <XIcon />);
				return;
			}
			if (commands.length === 0) return;
			const result = await backend.boardState.executeCommands(
				appId,
				boardId,
				commands,
			);
			await pushCommands(result);
			await board.refetch();

			if (awarenessRef.current) {
				awarenessRef.current.setLocalStateField("boardUpdate", Date.now());
			}

			return result;
		},
		[board.refetch, appId, boardId, pushCommands, version],
	);

	return {
		executeCommand,
		executeCommands,
		awarenessRef,
	};
}
