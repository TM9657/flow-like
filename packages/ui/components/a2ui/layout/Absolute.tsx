"use client";

import { Fragment } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps, RenderChildFn } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { AbsoluteComponent, BoundValue, Children } from "../types";

function getChildIds(children: Children | undefined): string[] {
	if (!children) return [];
	if ("explicitList" in children) return children.explicitList;
	return [];
}

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

export function A2UIAbsolute({
	component,
	style,
	renderChild,
}: ComponentProps<AbsoluteComponent> & { renderChild: RenderChildFn }) {
	const width = useResolved<string>(component.width);
	const height = useResolved<string>(component.height);
	const childIds = getChildIds(component.children);

	return (
		<div
			className={cn("relative", resolveStyle(style))}
			style={{
				width: width ?? "100%",
				height: height ?? "100%",
				...resolveInlineStyle(style),
			}}
		>
			{childIds.map((id) => (
				<Fragment key={id}>{renderChild(id)}</Fragment>
			))}
		</div>
	);
}
