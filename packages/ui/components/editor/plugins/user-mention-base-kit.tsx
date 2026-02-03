"use client";
import { createSlatePlugin } from "platejs";

import { UserMentionElementStatic } from "../ui/user-mention-node-static";

export const USER_MENTION_KEY = "user_mention";

export const BaseUserMentionPlugin = createSlatePlugin({
	key: USER_MENTION_KEY,
	node: {
		isElement: true,
		isInline: true,
		isVoid: true,
	},
});

export const BaseUserMentionKit = [
	BaseUserMentionPlugin.withComponent(UserMentionElementStatic),
];
