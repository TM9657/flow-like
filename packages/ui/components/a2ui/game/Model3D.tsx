"use client";

import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, Model3DComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

export function A2UIModel3D({
	component,
	style,
}: ComponentProps<Model3DComponent>) {
	const src = useResolved<string>(component.src);
	const position = useResolved<[number, number, number]>(component.position);
	const rotation = useResolved<[number, number, number]>(component.rotation);
	const scale = useResolved<number | [number, number, number]>(component.scale);

	return (
		<div
			className={cn("inline-block", resolveStyle(style))}
			style={resolveInlineStyle(style)}
			data-model-src={src}
			data-position={position?.join(",")}
			data-rotation={rotation?.join(",")}
			data-scale={typeof scale === "number" ? scale : scale?.join(",")}
		>
			<div className="p-2 text-xs text-muted-foreground bg-muted rounded">
				3D Model: {src?.split("/").pop() ?? "Unknown"}
			</div>
		</div>
	);
}
