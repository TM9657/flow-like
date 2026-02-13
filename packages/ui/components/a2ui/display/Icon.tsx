"use client";

import * as LucideIcons from "lucide-react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, IconComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

function toPascalCase(str: string): string {
	return str
		.split(/[-_\s]+/)
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
		.join("");
}

export function A2UIIcon({ component, style }: ComponentProps<IconComponent>) {
	const iconName = useResolved<string>(component.name);

	const IconComp = iconName
		? (LucideIcons as Record<string, any>)[toPascalCase(iconName)]
		: null;

	if (!IconComp) {
		return (
			<span
				className={cn(
					"inline-flex items-center justify-center",
					resolveStyle(style),
				)}
				style={resolveInlineStyle(style)}
			>
				?
			</span>
		);
	}

	return (
		<IconComp
			className={cn(resolveStyle(style))}
			style={{
				width: component.size ?? "1em",
				height: component.size ?? "1em",
				color: component.color,
				...resolveInlineStyle(style),
			}}
			strokeWidth={component.strokeWidth ?? 2}
		/>
	);
}
