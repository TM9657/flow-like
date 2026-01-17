"use client";

import { Fragment } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, CenterComponent } from "../types";

export function A2UICenter({
	component,
	style,
	renderChild,
}: ComponentProps<CenterComponent>) {
	const { resolve } = useData();

	const inline = component.inline
		? (resolve(component.inline) as boolean)
		: false;
	const children = resolveChildren(component, resolve);

	return (
		<div
			className={cn(
				inline ? "inline-flex" : "flex",
				"items-center justify-center",
				resolveStyle(style),
			)}
			style={resolveInlineStyle(style)}
		>
			{children.map((childId) => (
				<Fragment key={childId}>{renderChild(childId)}</Fragment>
			))}
		</div>
	);
}

function resolveChildren(
	component: CenterComponent,
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
