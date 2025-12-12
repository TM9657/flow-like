"use client";

import {
	BookOpen,
	Copy,
	LayoutGrid,
	Play,
	Redo2,
	Save,
	Search,
	Undo2,
	ZoomIn,
	ZoomOut,
} from "lucide-react";
import { useMemo } from "react";
import type { SpotlightItem } from "../state/spotlight-state";
import { useSpotlightGroup, useSpotlightItems } from "./use-spotlight";

export interface FlowBoardSpotlightOptions {
	boardId: string;
	boardName?: string;
	canUndo: boolean;
	canRedo: boolean;
	onUndo: () => void | Promise<void>;
	onRedo: () => void | Promise<void>;
	onOpenCatalog: () => void;
	onToggleVariables?: () => void;
	onFitView?: () => void;
	onZoomIn?: () => void;
	onZoomOut?: () => void;
	onSave?: () => void | Promise<void>;
	onRun?: () => void | Promise<void>;
	onDuplicate?: () => void | Promise<void>;
	enabled?: boolean;
}

export function useFlowBoardSpotlight({
	boardId,
	boardName,
	canUndo,
	canRedo,
	onUndo,
	onRedo,
	onOpenCatalog,
	onToggleVariables,
	onFitView,
	onZoomIn,
	onZoomOut,
	onSave,
	onRun,
	onDuplicate,
	enabled = true,
}: FlowBoardSpotlightOptions) {
	useSpotlightGroup({
		group: {
			id: "flow-board",
			label: `Flow: ${boardName || "Board"}`,
			priority: 250,
		},
		enabled,
	});

	const items = useMemo<SpotlightItem[]>(() => {
		const result: SpotlightItem[] = [];

		result.push({
			id: `flow-${boardId}-catalog`,
			type: "dynamic",
			label: "Open Node Catalog",
			description: "Browse and search available nodes",
			icon: Search,
			group: "flow-board",
			keywords: ["catalog", "nodes", "search", "add", "browse"],
			shortcut: "Tab",
			action: onOpenCatalog,
			priority: 300,
		});

		result.push({
			id: `flow-${boardId}-undo`,
			type: "dynamic",
			label: "Undo",
			description: "Undo the last change",
			icon: Undo2,
			group: "flow-board",
			keywords: ["undo", "revert", "back"],
			shortcut: "⌘Z",
			action: onUndo,
			disabled: !canUndo,
			priority: 290,
		});

		result.push({
			id: `flow-${boardId}-redo`,
			type: "dynamic",
			label: "Redo",
			description: "Redo the last undone change",
			icon: Redo2,
			group: "flow-board",
			keywords: ["redo", "forward"],
			shortcut: "⌘Y",
			action: onRedo,
			disabled: !canRedo,
			priority: 280,
		});

		if (onToggleVariables) {
			result.push({
				id: `flow-${boardId}-variables`,
				type: "dynamic",
				label: "Toggle Variables Panel",
				description: "Show or hide the variables panel",
				icon: BookOpen,
				group: "flow-board",
				keywords: ["variables", "panel", "toggle", "show", "hide"],
				action: onToggleVariables,
				priority: 270,
			});
		}

		if (onFitView) {
			result.push({
				id: `flow-${boardId}-fit-view`,
				type: "dynamic",
				label: "Fit to View",
				description: "Fit all nodes into the viewport",
				icon: LayoutGrid,
				group: "flow-board",
				keywords: ["fit", "view", "zoom", "center", "viewport"],
				action: onFitView,
				priority: 260,
			});
		}

		if (onZoomIn) {
			result.push({
				id: `flow-${boardId}-zoom-in`,
				type: "dynamic",
				label: "Zoom In",
				description: "Zoom into the board",
				icon: ZoomIn,
				group: "flow-board",
				keywords: ["zoom", "in", "magnify", "enlarge"],
				shortcut: "⌘+",
				action: onZoomIn,
				priority: 250,
			});
		}

		if (onZoomOut) {
			result.push({
				id: `flow-${boardId}-zoom-out`,
				type: "dynamic",
				label: "Zoom Out",
				description: "Zoom out of the board",
				icon: ZoomOut,
				group: "flow-board",
				keywords: ["zoom", "out", "shrink"],
				shortcut: "⌘-",
				action: onZoomOut,
				priority: 240,
			});
		}

		if (onSave) {
			result.push({
				id: `flow-${boardId}-save`,
				type: "dynamic",
				label: "Save Board",
				description: "Save the current board",
				icon: Save,
				group: "flow-board",
				keywords: ["save", "persist", "store"],
				shortcut: "⌘S",
				action: onSave,
				priority: 295,
			});
		}

		if (onRun) {
			result.push({
				id: `flow-${boardId}-run`,
				type: "dynamic",
				label: "Run Flow",
				description: "Execute the flow",
				icon: Play,
				group: "flow-board",
				keywords: ["run", "execute", "start", "play"],
				shortcut: "⌘R",
				action: onRun,
				priority: 285,
			});
		}

		if (onDuplicate) {
			result.push({
				id: `flow-${boardId}-duplicate`,
				type: "dynamic",
				label: "Duplicate Selection",
				description: "Duplicate selected nodes",
				icon: Copy,
				group: "flow-board",
				keywords: ["duplicate", "copy", "clone"],
				shortcut: "⌘D",
				action: onDuplicate,
				priority: 275,
			});
		}

		return result;
	}, [
		boardId,
		canUndo,
		canRedo,
		onUndo,
		onRedo,
		onOpenCatalog,
		onToggleVariables,
		onFitView,
		onZoomIn,
		onZoomOut,
		onSave,
		onRun,
		onDuplicate,
	]);

	useSpotlightItems({
		sourceId: `flow-board-${boardId}`,
		items,
		enabled,
	});
}
