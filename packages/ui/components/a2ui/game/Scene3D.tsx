"use client";

import { Fragment, Suspense } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps, RenderChildFn } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, Children, Scene3DComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

function getChildIds(children: Children | undefined): string[] {
	if (!children) return [];
	if ("explicitList" in children) return children.explicitList;
	return [];
}

export function A2UIScene3D({
	component,
	style,
	renderChild,
}: ComponentProps<Scene3DComponent> & { renderChild: RenderChildFn }) {
	const width = useResolved<string>(component.width) ?? "100%";
	const height = useResolved<string>(component.height) ?? "400px";
	const backgroundColor =
		useResolved<string>(component.backgroundColor) ?? "#1a1a2e";
	const childIds = getChildIds(component.children);

	return (
		<div
			className={cn("relative", resolveStyle(style))}
			style={{
				width,
				height,
				backgroundColor,
				...resolveInlineStyle(style),
			}}
		>
			<Suspense fallback={<Scene3DFallback />}>
				<Scene3DCanvas component={component}>
					{childIds.map((id) => (
						<Fragment key={id}>{renderChild(id)}</Fragment>
					))}
				</Scene3DCanvas>
			</Suspense>
		</div>
	);
}

function Scene3DFallback() {
	return (
		<div className="flex items-center justify-center w-full h-full text-muted-foreground">
			Loading 3D Scene...
		</div>
	);
}

function Scene3DCanvas({
	component,
	children,
}: {
	component: Scene3DComponent;
	children: React.ReactNode;
}) {
	const { resolve } = useData();
	const cameraType = component.cameraType
		? (resolve(component.cameraType) as string)
		: "perspective";
	const cameraPosition = component.cameraPosition
		? (resolve(component.cameraPosition) as number[])
		: [0, 0, 5];

	return (
		<div className="flex items-center justify-center w-full h-full text-muted-foreground">
			<div className="text-center">
				<p className="text-sm">3D Scene</p>
				<p className="text-xs opacity-60">Camera: {cameraType}</p>
				<p className="text-xs opacity-60">
					Position: {cameraPosition?.join(", ") ?? "0, 0, 5"}
				</p>
				{children}
			</div>
		</div>
	);
}
