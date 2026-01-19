"use client";

import { DndProvider } from "react-dnd";
import { HTML5Backend } from "react-dnd-html5-backend";
import { TouchBackend } from "react-dnd-touch-backend";

import { DndPlugin } from "@platejs/dnd";
import { PlaceholderPlugin } from "@platejs/media/react";

import { isTauri } from "../../../lib/platform";
import { BlockDraggable } from "../ui/block-draggable";

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
					? {
							enableMouseEvents: true,
							delayTouchStart: 120,
							ignoreContextMenu: true,
						}
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
