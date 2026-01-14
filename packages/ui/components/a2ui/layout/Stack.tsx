"use client";

import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, StackComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

export function A2UIStack({
	component,
	style,
	renderChild,
}: ComponentProps<StackComponent>) {
	const { resolve } = useData();

	const alignMap: Record<string, string> = {
		start: "items-start",
		center: "items-center",
		end: "items-end",
		stretch: "items-stretch",
	};

	const align = useResolved<string>(component.align);
	const width = useResolved<string>(component.width);
	const height = useResolved<string>(component.height);

	const children = resolveChildren(component, resolve);

	// Build inline styles from component props
	const inlineStyles = {
		...resolveInlineStyle(style),
		...(width && { width }),
		...(height && { height }),
	};

	return (
		<div
			className={cn(
				"relative",
				align && alignMap[align],
				// Ensure stack has minimum dimensions when empty
				children.length === 0 && "min-h-[100px] min-w-[100px]",
				resolveStyle(style),
			)}
			style={inlineStyles}
		>
			{children.map((childId, index) => (
				<div
					key={childId}
					className="absolute inset-0"
					style={{ zIndex: index }}
				>
					{renderChild(childId)}
				</div>
			))}
		</div>
	);
}

function resolveChildren(
	component: StackComponent,
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
