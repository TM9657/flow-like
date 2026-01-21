"use client";

import { useMemo } from "react";
import { cn } from "../../lib";

export interface SnapGuideLine {
	type: "horizontal" | "vertical";
	position: number;
	start: number;
	end: number;
	kind: "edge" | "center" | "spacing";
}

export interface ComponentBounds {
	id: string;
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface SnapGuidesProps {
	guides: SnapGuideLine[];
	zoom: number;
	pan: { x: number; y: number };
	className?: string;
}

const SNAP_THRESHOLD = 5;

export function SnapGuides({ guides, zoom, pan, className }: SnapGuidesProps) {
	if (guides.length === 0) return null;

	return (
		<svg
			className={cn(
				"absolute inset-0 pointer-events-none overflow-visible z-50",
				className,
			)}
			style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
		>
			{guides.map((guide, index) => (
				<g key={index}>
					{guide.type === "vertical" ? (
						<line
							x1={guide.position}
							y1={guide.start}
							x2={guide.position}
							y2={guide.end}
							stroke={guide.kind === "center" ? "#f43f5e" : "#3b82f6"}
							strokeWidth={1 / zoom}
							strokeDasharray={
								guide.kind === "spacing" ? `${4 / zoom} ${4 / zoom}` : "none"
							}
						/>
					) : (
						<line
							x1={guide.start}
							y1={guide.position}
							x2={guide.end}
							y2={guide.position}
							stroke={guide.kind === "center" ? "#f43f5e" : "#3b82f6"}
							strokeWidth={1 / zoom}
							strokeDasharray={
								guide.kind === "spacing" ? `${4 / zoom} ${4 / zoom}` : "none"
							}
						/>
					)}
				</g>
			))}
		</svg>
	);
}

export function useSnapGuides(
	draggingBounds: ComponentBounds | null,
	otherBounds: ComponentBounds[],
	canvasSize: { width: number; height: number },
): {
	guides: SnapGuideLine[];
	snappedPosition: { x: number; y: number } | null;
} {
	return useMemo(() => {
		if (!draggingBounds) {
			return { guides: [], snappedPosition: null };
		}

		const guides: SnapGuideLine[] = [];
		let snappedX: number | null = null;
		let snappedY: number | null = null;

		const dragLeft = draggingBounds.x;
		const dragRight = draggingBounds.x + draggingBounds.width;
		const dragTop = draggingBounds.y;
		const dragBottom = draggingBounds.y + draggingBounds.height;
		const dragCenterX = draggingBounds.x + draggingBounds.width / 2;
		const dragCenterY = draggingBounds.y + draggingBounds.height / 2;

		const canvasCenterX = canvasSize.width / 2;
		const canvasCenterY = canvasSize.height / 2;
		if (Math.abs(dragCenterX - canvasCenterX) < SNAP_THRESHOLD) {
			snappedX = canvasCenterX - draggingBounds.width / 2;
			guides.push({
				type: "vertical",
				position: canvasCenterX,
				start: 0,
				end: canvasSize.height,
				kind: "center",
			});
		}
		if (Math.abs(dragCenterY - canvasCenterY) < SNAP_THRESHOLD) {
			snappedY = canvasCenterY - draggingBounds.height / 2;
			guides.push({
				type: "horizontal",
				position: canvasCenterY,
				start: 0,
				end: canvasSize.width,
				kind: "center",
			});
		}

		for (const other of otherBounds) {
			if (other.id === draggingBounds.id) continue;

			const otherLeft = other.x;
			const otherRight = other.x + other.width;
			const otherTop = other.y;
			const otherBottom = other.y + other.height;
			const otherCenterX = other.x + other.width / 2;
			const otherCenterY = other.y + other.height / 2;

			if (Math.abs(dragLeft - otherLeft) < SNAP_THRESHOLD) {
				snappedX ??= otherLeft;
				guides.push({
					type: "vertical",
					position: otherLeft,
					start: Math.min(dragTop, otherTop),
					end: Math.max(dragBottom, otherBottom),
					kind: "edge",
				});
			}
			if (Math.abs(dragRight - otherRight) < SNAP_THRESHOLD) {
				snappedX ??= otherRight - draggingBounds.width;
				guides.push({
					type: "vertical",
					position: otherRight,
					start: Math.min(dragTop, otherTop),
					end: Math.max(dragBottom, otherBottom),
					kind: "edge",
				});
			}
			if (Math.abs(dragLeft - otherRight) < SNAP_THRESHOLD) {
				snappedX ??= otherRight;
				guides.push({
					type: "vertical",
					position: otherRight,
					start: Math.min(dragTop, otherTop),
					end: Math.max(dragBottom, otherBottom),
					kind: "edge",
				});
			}
			if (Math.abs(dragRight - otherLeft) < SNAP_THRESHOLD) {
				snappedX ??= otherLeft - draggingBounds.width;
				guides.push({
					type: "vertical",
					position: otherLeft,
					start: Math.min(dragTop, otherTop),
					end: Math.max(dragBottom, otherBottom),
					kind: "edge",
				});
			}
			if (Math.abs(dragCenterX - otherCenterX) < SNAP_THRESHOLD) {
				snappedX ??= otherCenterX - draggingBounds.width / 2;
				guides.push({
					type: "vertical",
					position: otherCenterX,
					start: Math.min(dragTop, otherTop),
					end: Math.max(dragBottom, otherBottom),
					kind: "center",
				});
			}

			if (Math.abs(dragTop - otherTop) < SNAP_THRESHOLD) {
				snappedY ??= otherTop;
				guides.push({
					type: "horizontal",
					position: otherTop,
					start: Math.min(dragLeft, otherLeft),
					end: Math.max(dragRight, otherRight),
					kind: "edge",
				});
			}
			if (Math.abs(dragBottom - otherBottom) < SNAP_THRESHOLD) {
				snappedY ??= otherBottom - draggingBounds.height;
				guides.push({
					type: "horizontal",
					position: otherBottom,
					start: Math.min(dragLeft, otherLeft),
					end: Math.max(dragRight, otherRight),
					kind: "edge",
				});
			}
			if (Math.abs(dragTop - otherBottom) < SNAP_THRESHOLD) {
				snappedY ??= otherBottom;
				guides.push({
					type: "horizontal",
					position: otherBottom,
					start: Math.min(dragLeft, otherLeft),
					end: Math.max(dragRight, otherRight),
					kind: "edge",
				});
			}
			if (Math.abs(dragBottom - otherTop) < SNAP_THRESHOLD) {
				snappedY ??= otherTop - draggingBounds.height;
				guides.push({
					type: "horizontal",
					position: otherTop,
					start: Math.min(dragLeft, otherLeft),
					end: Math.max(dragRight, otherRight),
					kind: "edge",
				});
			}
			if (Math.abs(dragCenterY - otherCenterY) < SNAP_THRESHOLD) {
				snappedY ??= otherCenterY - draggingBounds.height / 2;
				guides.push({
					type: "horizontal",
					position: otherCenterY,
					start: Math.min(dragLeft, otherLeft),
					end: Math.max(dragRight, otherRight),
					kind: "center",
				});
			}
		}

		return {
			guides,
			snappedPosition:
				snappedX !== null || snappedY !== null
					? { x: snappedX ?? draggingBounds.x, y: snappedY ?? draggingBounds.y }
					: null,
		};
	}, [draggingBounds, otherBounds, canvasSize]);
}
