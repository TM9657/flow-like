"use client";

import { Fragment } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { AspectRatioComponent } from "../types";

export function A2UIAspectRatio({
	component,
	style,
	renderChild,
}: ComponentProps<AspectRatioComponent>) {
	const { resolve } = useData();
	const children = resolveChildren(component, resolve);
	const ratio = (resolve(component.ratio) as number) || 1;

	return (
		<div
			className={cn("relative w-full", resolveStyle(style))}
			style={{
				paddingBottom: `${(1 / ratio) * 100}%`,
				...resolveInlineStyle(style),
			}}
		>
			<div className="absolute inset-0">
				{children.map((childId) => (
					<Fragment key={childId}>{renderChild(childId)}</Fragment>
				))}
			</div>
		</div>
	);
}

function resolveChildren(
	component: AspectRatioComponent,
	resolve: (boundValue: any) => unknown,
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
