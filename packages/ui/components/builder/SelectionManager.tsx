"use client";

import {
	type MouseEvent,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { cn } from "../../lib";

export interface BuilderSelectionRect {
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface SelectionManagerProps {
	className?: string;
	zoom: number;
	pan: { x: number; y: number };
	onSelectionChange?: (rect: BuilderSelectionRect | null) => void;
	onSelectionComplete?: (rect: BuilderSelectionRect) => void;
	children?: React.ReactNode;
	disabled?: boolean;
}

export function SelectionManager({
	className,
	zoom,
	pan,
	onSelectionChange,
	onSelectionComplete,
	children,
	disabled = false,
}: SelectionManagerProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const [isSelecting, setIsSelecting] = useState(false);
	const [startPoint, setStartPoint] = useState<{ x: number; y: number } | null>(
		null,
	);
	const [selectionRect, setSelectionRect] =
		useState<BuilderSelectionRect | null>(null);

	const getCanvasPoint = useCallback(
		(clientX: number, clientY: number) => {
			if (!containerRef.current) return { x: 0, y: 0 };
			const rect = containerRef.current.getBoundingClientRect();
			return {
				x: (clientX - rect.left - pan.x) / zoom,
				y: (clientY - rect.top - pan.y) / zoom,
			};
		},
		[zoom, pan],
	);

	const handleMouseDown = useCallback(
		(event: MouseEvent) => {
			if (disabled || event.button !== 0) return;
			if ((event.target as HTMLElement).closest("[data-selectable]")) return;

			const point = getCanvasPoint(event.clientX, event.clientY);
			setStartPoint(point);
			setIsSelecting(true);
			setSelectionRect(null);
		},
		[disabled, getCanvasPoint],
	);

	const handleMouseMove = useCallback(
		(event: MouseEvent) => {
			if (!isSelecting || !startPoint) return;

			const currentPoint = getCanvasPoint(event.clientX, event.clientY);
			const rect: BuilderSelectionRect = {
				x: Math.min(startPoint.x, currentPoint.x),
				y: Math.min(startPoint.y, currentPoint.y),
				width: Math.abs(currentPoint.x - startPoint.x),
				height: Math.abs(currentPoint.y - startPoint.y),
			};

			setSelectionRect(rect);
			onSelectionChange?.(rect);
		},
		[isSelecting, startPoint, getCanvasPoint, onSelectionChange],
	);

	const handleMouseUp = useCallback(() => {
		if (
			isSelecting &&
			selectionRect &&
			selectionRect.width > 5 &&
			selectionRect.height > 5
		) {
			onSelectionComplete?.(selectionRect);
		}
		setIsSelecting(false);
		setStartPoint(null);
		setSelectionRect(null);
		onSelectionChange?.(null);
	}, [isSelecting, selectionRect, onSelectionComplete, onSelectionChange]);

	useEffect(() => {
		const handleGlobalMouseUp = () => handleMouseUp();
		window.addEventListener("mouseup", handleGlobalMouseUp);
		return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
	}, [handleMouseUp]);

	return (
		<div
			ref={containerRef}
			className={cn("relative", className)}
			onMouseDown={handleMouseDown}
			onMouseMove={handleMouseMove}
		>
			{children}

			{selectionRect && (
				<div
					className="absolute border-2 border-blue-500 bg-blue-500/10 pointer-events-none z-50"
					style={{
						left: selectionRect.x * zoom + pan.x,
						top: selectionRect.y * zoom + pan.y,
						width: selectionRect.width * zoom,
						height: selectionRect.height * zoom,
					}}
				/>
			)}
		</div>
	);
}

export function useMarqueeSelection(
	componentBounds: Map<string, DOMRect>,
	selectionRect: BuilderSelectionRect | null,
): string[] {
	const selectedIds: string[] = [];
	if (!selectionRect) return selectedIds;

	for (const [id, bounds] of componentBounds) {
		if (
			bounds.left < selectionRect.x + selectionRect.width &&
			bounds.right > selectionRect.x &&
			bounds.top < selectionRect.y + selectionRect.height &&
			bounds.bottom > selectionRect.y
		) {
			selectedIds.push(id);
		}
	}

	return selectedIds;
}
