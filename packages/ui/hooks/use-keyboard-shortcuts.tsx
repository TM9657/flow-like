import type { UseQueryResult } from "@tanstack/react-query";
import { Redo2Icon, Undo2Icon, XIcon } from "lucide-react";
import { useCallback, useEffect } from "react";
import { toastError, toastSuccess } from "../lib/messages";
import type { IBoard } from "../lib/schema/flow/board";
import type { INode } from "../lib/schema/flow/node";
import { useBackend } from "../state/backend-state";

interface UseKeyboardShortcutsProps {
	board: UseQueryResult<IBoard>;
	catalog: UseQueryResult<INode[]>;
	version: [number, number, number] | undefined;
	appId: string;
	boardId: string;
	mousePosition: { x: number; y: number };
	placeNode: (
		node: INode,
		position?: { x: number; y: number },
	) => Promise<void>;
	undo: () => Promise<any>;
	redo: () => Promise<any>;
}

export function useKeyboardShortcuts({
	board,
	catalog,
	version,
	appId,
	boardId,
	mousePosition,
	placeNode,
	undo,
	redo,
}: UseKeyboardShortcutsProps) {
	const backend = useBackend();

	const placeNodeShortcut = useCallback(
		async (node: INode) => {
			await placeNode(node, {
				x: mousePosition.x,
				y: mousePosition.y,
			});
		},
		[mousePosition, placeNode],
	);

	const shortcutHandler = useCallback(
		async (event: KeyboardEvent) => {
			if (event.repeat) return;

			const target = event.target as HTMLElement;
			if (
				target.tagName === "INPUT" ||
				target.tagName === "TEXTAREA" ||
				target.isContentEditable
			) {
				return;
			}

			// Undo
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "z" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const stack = await undo();
				if (stack) await backend.boardState.undoBoard(appId, boardId, stack);
				toastSuccess("Undo", <Undo2Icon className="w-4 h-4" />);
				await board.refetch();
				return;
			}

			// Redo
			if ((event.metaKey || event.ctrlKey) && event.key === "y") {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const stack = await redo();
				if (stack) await backend.boardState.redoBoard(appId, boardId, stack);
				toastSuccess("Redo", <Redo2Icon className="w-4 h-4" />);
				await board.refetch();
				return;
			}

			// Place Branch
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "b" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const node = catalog.data?.find(
					(node) => node.name === "control_branch",
				);
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}

			// Place For Each
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "f" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const node = catalog.data?.find(
					(node) => node.name === "control_for_each",
				);
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}

			// Place Log Info
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "p" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const node = catalog.data?.find((node) => node.name === "log_info");
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
				return;
			}

			// Place Reroute
			if (
				(event.metaKey || event.ctrlKey) &&
				event.key === "s" &&
				!event.shiftKey
			) {
				event.preventDefault();
				event.stopPropagation();
				if (typeof version !== "undefined") {
					toastError("Cannot change old version", <XIcon />);
					return;
				}
				const node = catalog.data?.find((node) => node.name === "reroute");
				if (!node) return;
				await placeNodeShortcut(node);
				await board.refetch();
			}
		},
		[
			boardId,
			board,
			backend,
			version,
			catalog,
			placeNodeShortcut,
			undo,
			redo,
			appId,
		],
	);

	useEffect(() => {
		document.addEventListener("keydown", shortcutHandler);
		return () => {
			document.removeEventListener("keydown", shortcutHandler);
		};
	}, [shortcutHandler]);
}
