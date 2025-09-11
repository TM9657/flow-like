"use client";

import { DndProvider } from "react-dnd";
import { HTML5Backend } from "react-dnd-html5-backend";
import { TouchBackend } from "react-dnd-touch-backend";

import { DndPlugin } from "@platejs/dnd";
import { PlaceholderPlugin } from "@platejs/media/react";

import { BlockDraggable } from "../ui/block-draggable";

// Detect Tauri environment (WebView) where HTML5 DnD backend is unreliable
const isTauri = (): boolean => {
	// Common Tauri globals
	// __TAURI__ in Tauri v1, __TAURI_INTERNALS__ / __TAURI_IPC__ in some builds
	if (typeof window === "undefined") return false;
	const w = window as any;
	return !!(w.__TAURI__ || w.__TAURI_IPC__ || w.__TAURI_INTERNALS__);
};

export const DndKit = [
	DndPlugin.configure({
		options: {
			enableScroller: true,
			onDropFiles: ({ dragItem, editor, target }) => {
				editor
					.getTransforms(PlaceholderPlugin)
					.insert.media(dragItem.files, { at: target, nextBlock: false });
			},
		},
		render: {
			aboveNodes: BlockDraggable,
				aboveSlate: (props) => {
					const backend = isTauri() ? TouchBackend : HTML5Backend;
					const options = isTauri()
						? { enableMouseEvents: true, delayTouchStart: 120, ignoreContextMenu: true }
						: undefined;
					return (
						// react-dnd types allow options; cast if needed to satisfy TS in this environment
						<DndProvider backend={backend as any} options={options as any}>
							{props.children}
						</DndProvider>
					);
				},
		},
	}),
];
