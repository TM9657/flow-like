"use client";
import { createSlatePlugin } from "platejs";

import { FocusNodeElementStatic } from "../ui/focus-node-static";

export const FOCUS_NODE_KEY = "focus_node";

export const BaseFocusNodePlugin = createSlatePlugin({
	key: FOCUS_NODE_KEY,
	node: {
		isElement: true,
		isInline: true,
		isVoid: true,
	},
});

export const BaseFocusNodeKit = [
	BaseFocusNodePlugin.withComponent(FocusNodeElementStatic),
];
