"use client";

import type { ElementType } from "react";
import { Fragment } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, BoxComponent } from "../types";

export function A2UIBox({
	component,
	style,
	renderChild,
}: ComponentProps<BoxComponent>) {
	const { resolve } = useData();

	const as = component.as ? (resolve(component.as) as string) : "div";
	const children = resolveChildren(component, resolve);

	const Tag = as as ElementType;

	return (
		<Tag className={cn(resolveStyle(style))} style={resolveInlineStyle(style)}>
			{children.map((childId) => (
				<Fragment key={childId}>{renderChild(childId)}</Fragment>
			))}
		</Tag>
	);
}

function resolveChildren(
	component: BoxComponent,
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
