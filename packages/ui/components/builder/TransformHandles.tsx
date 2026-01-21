"use client";

import { type MouseEvent, useCallback, useRef, useState } from "react";
import { cn } from "../../lib";

export type HandlePosition =
	| "top"
	| "right"
	| "bottom"
	| "left"
	| "topLeft"
	| "topRight"
	| "bottomLeft"
	| "bottomRight"
	| "rotate";

export interface TransformBounds {
	x: number;
	y: number;
	width: number;
	height: number;
	rotation?: number;
}

export interface TransformHandlesProps {
	bounds: TransformBounds;
	zoom: number;
	onTransformStart?: (handle: HandlePosition) => void;
	onTransform?: (delta: Partial<TransformBounds>) => void;
	onTransformEnd?: () => void;
	showRotateHandle?: boolean;
	disabled?: boolean;
	className?: string;
}

const HANDLE_SIZE = 8;
const ROTATE_HANDLE_OFFSET = 24;

export function TransformHandles({
	bounds,
	zoom,
	onTransformStart,
	onTransform,
	onTransformEnd,
	showRotateHandle = true,
	disabled = false,
	className,
}: TransformHandlesProps) {
	const [activeHandle, setActiveHandle] = useState<HandlePosition | null>(null);
	const startPosRef = useRef({ x: 0, y: 0 });
	const startBoundsRef = useRef<TransformBounds>(bounds);

	const handleMouseDown = useCallback(
		(event: MouseEvent, handle: HandlePosition) => {
			if (disabled) return;
			event.stopPropagation();
			event.preventDefault();

			setActiveHandle(handle);
			startPosRef.current = { x: event.clientX, y: event.clientY };
			startBoundsRef.current = { ...bounds };
			onTransformStart?.(handle);

			const handleMouseMove = (e: globalThis.MouseEvent) => {
				const deltaX = (e.clientX - startPosRef.current.x) / zoom;
				const deltaY = (e.clientY - startPosRef.current.y) / zoom;
				const start = startBoundsRef.current;

				const delta: Partial<TransformBounds> = {};

				switch (handle) {
					case "top":
						delta.y = start.y + deltaY;
						delta.height = start.height - deltaY;
						break;
					case "bottom":
						delta.height = start.height + deltaY;
						break;
					case "left":
						delta.x = start.x + deltaX;
						delta.width = start.width - deltaX;
						break;
					case "right":
						delta.width = start.width + deltaX;
						break;
					case "topLeft":
						delta.x = start.x + deltaX;
						delta.y = start.y + deltaY;
						delta.width = start.width - deltaX;
						delta.height = start.height - deltaY;
						break;
					case "topRight":
						delta.y = start.y + deltaY;
						delta.width = start.width + deltaX;
						delta.height = start.height - deltaY;
						break;
					case "bottomLeft":
						delta.x = start.x + deltaX;
						delta.width = start.width - deltaX;
						delta.height = start.height + deltaY;
						break;
					case "bottomRight":
						delta.width = start.width + deltaX;
						delta.height = start.height + deltaY;
						break;
					case "rotate":
						const centerX = start.x + start.width / 2;
						const centerY = start.y + start.height / 2;
						const angle = Math.atan2(
							e.clientY / zoom - centerY,
							e.clientX / zoom - centerX,
						);
						delta.rotation = (angle * 180) / Math.PI + 90;
						break;
				}

				if (delta.width !== undefined && delta.width < 10) delta.width = 10;
				if (delta.height !== undefined && delta.height < 10) delta.height = 10;

				onTransform?.(delta);
			};

			const handleMouseUp = () => {
				setActiveHandle(null);
				onTransformEnd?.();
				window.removeEventListener("mousemove", handleMouseMove);
				window.removeEventListener("mouseup", handleMouseUp);
			};

			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", handleMouseUp);
		},
		[disabled, bounds, zoom, onTransformStart, onTransform, onTransformEnd],
	);

	const renderHandle = (
		position: HandlePosition,
		x: number,
		y: number,
		cursor: string,
	) => (
		<div
			key={position}
			className={cn(
				"absolute bg-white border-2 border-blue-500 rounded-sm hover:bg-blue-100 transition-colors",
				activeHandle === position && "bg-blue-200",
				disabled && "cursor-not-allowed opacity-50",
			)}
			style={{
				left: x - HANDLE_SIZE / 2,
				top: y - HANDLE_SIZE / 2,
				width: HANDLE_SIZE,
				height: HANDLE_SIZE,
				cursor: disabled ? "not-allowed" : cursor,
			}}
			onMouseDown={(e) => handleMouseDown(e, position)}
		/>
	);

	const scaledWidth = bounds.width * zoom;
	const scaledHeight = bounds.height * zoom;

	return (
		<div
			className={cn("absolute pointer-events-none", className)}
			style={{
				left: bounds.x * zoom,
				top: bounds.y * zoom,
				width: scaledWidth,
				height: scaledHeight,
				transform: bounds.rotation
					? `rotate(${bounds.rotation}deg)`
					: undefined,
				transformOrigin: "center center",
			}}
		>
			<div className="absolute inset-0 border-2 border-blue-500 pointer-events-none" />

			<div className="pointer-events-auto">
				{renderHandle("topLeft", 0, 0, "nwse-resize")}
				{renderHandle("top", scaledWidth / 2, 0, "ns-resize")}
				{renderHandle("topRight", scaledWidth, 0, "nesw-resize")}
				{renderHandle("left", 0, scaledHeight / 2, "ew-resize")}
				{renderHandle("right", scaledWidth, scaledHeight / 2, "ew-resize")}
				{renderHandle("bottomLeft", 0, scaledHeight, "nesw-resize")}
				{renderHandle("bottom", scaledWidth / 2, scaledHeight, "ns-resize")}
				{renderHandle("bottomRight", scaledWidth, scaledHeight, "nwse-resize")}

				{showRotateHandle && (
					<>
						<div
							className="absolute border-l-2 border-blue-500"
							style={{
								left: scaledWidth / 2,
								top: -ROTATE_HANDLE_OFFSET,
								height: ROTATE_HANDLE_OFFSET - HANDLE_SIZE / 2,
							}}
						/>
						<div
							className={cn(
								"absolute w-4 h-4 rounded-full bg-white border-2 border-blue-500 hover:bg-blue-100",
								activeHandle === "rotate" && "bg-blue-200",
								disabled && "cursor-not-allowed opacity-50",
							)}
							style={{
								left: scaledWidth / 2 - 8,
								top: -ROTATE_HANDLE_OFFSET - 8,
								cursor: disabled ? "not-allowed" : "grab",
							}}
							onMouseDown={(e) => handleMouseDown(e, "rotate")}
						>
							<svg
								className="w-full h-full p-0.5 text-blue-500"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								strokeWidth="2"
							>
								<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" />
								<path d="M21 3v5h-5" />
							</svg>
						</div>
					</>
				)}
			</div>
		</div>
	);
}
