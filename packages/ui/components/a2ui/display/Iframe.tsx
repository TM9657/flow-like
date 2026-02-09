"use client";

import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { BoundValue, IframeComponent } from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

export function A2UIIframe({
	component,
	style,
}: ComponentProps<IframeComponent>) {
	const src = useResolved<string>(component.src);
	const width = useResolved<string>(component.width) ?? "100%";
	const height = useResolved<string>(component.height) ?? "400px";
	const title = useResolved<string>(component.title) ?? "Embedded content";
	const sandbox = useResolved<string>(component.sandbox);
	const allow = useResolved<string>(component.allow);
	const loading = useResolved<"lazy" | "eager">(component.loading);
	const referrerPolicy = useResolved<string>(component.referrerPolicy);
	const border = useResolved<boolean>(component.border);

	if (!src) {
		return (
			<div
				className={cn(
					"flex items-center justify-center bg-muted text-muted-foreground border rounded",
					resolveStyle(style),
				)}
				style={{ ...resolveInlineStyle(style), width, height }}
			>
				No URL provided
			</div>
		);
	}

	return (
		<iframe
			src={src}
			title={title}
			width={width}
			height={height}
			sandbox={sandbox}
			allow={allow}
			loading={loading ?? "lazy"}
			referrerPolicy={referrerPolicy as React.HTMLAttributeReferrerPolicy}
			className={cn(
				border ? "border rounded" : "border-0",
				resolveStyle(style),
			)}
			style={resolveInlineStyle(style)}
		/>
	);
}
