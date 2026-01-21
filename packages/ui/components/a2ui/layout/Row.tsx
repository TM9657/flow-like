"use client";

import { Fragment } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, RowComponent } from "../types";

function useResolved<T>(
	resolve: (bv: BoundValue) => unknown,
	boundValue: BoundValue | undefined,
): T | undefined {
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

export function A2UIRow({
	component,
	style,
	renderChild,
}: ComponentProps<RowComponent>) {
	const { resolve } = useData();

	const alignMap: Record<string, string> = {
		start: "items-start",
		center: "items-center",
		end: "items-end",
		stretch: "items-stretch",
		baseline: "items-baseline",
	};

	const justifyMap: Record<string, string> = {
		start: "justify-start",
		center: "justify-center",
		end: "justify-end",
		between: "justify-between",
		around: "justify-around",
		evenly: "justify-evenly",
	};

	const gap = useResolved<string>(resolve, component.gap);
	const align = useResolved<string>(resolve, component.align);
	const justify = useResolved<string>(resolve, component.justify);
	const wrap = useResolved<boolean>(resolve, component.wrap);
	const reverse = useResolved<boolean>(resolve, component.reverse);

	const children = resolveChildren(component, resolve);

	return (
		<div
			className={cn(
				"flex flex-row",
				align && alignMap[align],
				justify && justifyMap[justify],
				wrap && "flex-wrap",
				reverse && "flex-row-reverse",
				resolveStyle(style),
			)}
			style={{
				gap,
				...resolveInlineStyle(style),
			}}
		>
			{children.map((childId) => (
				<Fragment key={childId}>{renderChild(childId)}</Fragment>
			))}
		</div>
	);
}

function resolveChildren(
	component: RowComponent,
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
