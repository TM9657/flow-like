"use client";

import { Fragment } from "react";
import { cn } from "../../../lib/utils";
import { ScrollArea } from "../../ui/scroll-area";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, ScrollAreaComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

export function A2UIScrollArea({
	component,
	style,
	renderChild,
}: ComponentProps<ScrollAreaComponent>) {
	const { resolve } = useData();
	const children = resolveChildren(component, resolve);
	const direction = useResolved<string>(component.direction);

	const scrollClass =
		direction === "horizontal"
			? "overflow-x-auto overflow-y-hidden"
			: direction === "both"
				? "overflow-auto"
				: "overflow-y-auto overflow-x-hidden";

	return (
		<ScrollArea
			className={cn("h-full w-full", scrollClass, resolveStyle(style))}
			style={resolveInlineStyle(style)}
		>
			{children.map((childId) => (
				<Fragment key={childId}>{renderChild(childId)}</Fragment>
			))}
		</ScrollArea>
	);
}

function resolveChildren(
	component: ScrollAreaComponent,
	resolve: (boundValue: BoundValue) => unknown,
): string[] {
	if (!component.children) return [];

	if ("explicitList" in component.children) {
		return component.children.explicitList;
	}

	if ("template" in component.children) {
		const { template } = component.children;
		const items = resolve({ path: template.dataPath }) as unknown[];
		if (!Array.isArray(items)) return [];
		return items.map((_, i) => `${template.templateComponentId}[${i}]`);
	}

	return [];
}
