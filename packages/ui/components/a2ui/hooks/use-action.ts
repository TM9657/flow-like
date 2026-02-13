"use client";

import { useCallback } from "react";
import { useActions } from "../ActionHandler";
import type { Action } from "../types";

export function useAction(action: Action | string | undefined) {
	const { trigger } = useActions();

	return useCallback(
		(additionalContext: Record<string, unknown> = {}) => {
			if (!action) return;
			trigger(action, additionalContext);
		},
		[action, trigger],
	);
}

export function useActionCallback<T extends unknown[]>(
	action: Action | string | undefined,
	contextBuilder?: (...args: T) => Record<string, unknown>,
) {
	const { trigger } = useActions();

	return useCallback(
		(...args: T) => {
			if (!action) return;
			const context = contextBuilder ? contextBuilder(...args) : {};
			trigger(action, context);
		},
		[action, trigger, contextBuilder],
	);
}
