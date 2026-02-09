"use client";

import { type DragEvent, type ReactNode, useCallback, useRef } from "react";
import { cn } from "../../lib";

export interface DragSourceProps {
	componentType: string;
	children: ReactNode;
	className?: string;
	disabled?: boolean;
	onDragStart?: (type: string) => void;
	onDragEnd?: () => void;
}

export function DragSource({
	componentType,
	children,
	className,
	disabled = false,
	onDragStart,
	onDragEnd,
}: DragSourceProps) {
	const dragImageRef = useRef<HTMLDivElement>(null);

	const handleDragStart = useCallback(
		(event: DragEvent) => {
			if (disabled) {
				event.preventDefault();
				return;
			}

			event.dataTransfer.setData("application/x-component-type", componentType);
			event.dataTransfer.effectAllowed = "copy";

			if (dragImageRef.current) {
				event.dataTransfer.setDragImage(dragImageRef.current, 30, 20);
			}

			onDragStart?.(componentType);
		},
		[componentType, disabled, onDragStart],
	);

	const handleDragEnd = useCallback(() => {
		onDragEnd?.();
	}, [onDragEnd]);

	return (
		<>
			<div
				draggable={!disabled}
				onDragStart={handleDragStart}
				onDragEnd={handleDragEnd}
				className={cn(
					"cursor-grab active:cursor-grabbing select-none",
					disabled && "cursor-not-allowed opacity-50",
					className,
				)}
			>
				{children}
			</div>

			<div
				ref={dragImageRef}
				className="fixed -left-[9999px] px-3 py-1.5 bg-blue-500 text-white text-sm rounded shadow-lg pointer-events-none"
			>
				{componentType}
			</div>
		</>
	);
}

export interface DropTargetProps {
	children: ReactNode;
	className?: string;
	onDrop?: (componentType: string, position: { x: number; y: number }) => void;
	onDragOver?: (position: { x: number; y: number }) => void;
	onDragLeave?: () => void;
	disabled?: boolean;
}

export function DropTarget({
	children,
	className,
	onDrop,
	onDragOver,
	onDragLeave,
	disabled = false,
}: DropTargetProps) {
	const containerRef = useRef<HTMLDivElement>(null);

	const handleDragOver = useCallback(
		(event: DragEvent) => {
			if (disabled) return;
			if (!event.dataTransfer.types.includes("application/x-component-type"))
				return;

			event.preventDefault();
			event.dataTransfer.dropEffect = "copy";

			if (containerRef.current && onDragOver) {
				const rect = containerRef.current.getBoundingClientRect();
				onDragOver({
					x: event.clientX - rect.left,
					y: event.clientY - rect.top,
				});
			}
		},
		[disabled, onDragOver],
	);

	const handleDrop = useCallback(
		(event: DragEvent) => {
			if (disabled) return;

			const componentType = event.dataTransfer.getData(
				"application/x-component-type",
			);
			if (!componentType || !containerRef.current || !onDrop) return;

			event.preventDefault();

			const rect = containerRef.current.getBoundingClientRect();
			onDrop(componentType, {
				x: event.clientX - rect.left,
				y: event.clientY - rect.top,
			});
		},
		[disabled, onDrop],
	);

	const handleDragLeave = useCallback(
		(event: DragEvent) => {
			if (
				containerRef.current &&
				!containerRef.current.contains(event.relatedTarget as Node)
			) {
				onDragLeave?.();
			}
		},
		[onDragLeave],
	);

	return (
		<div
			ref={containerRef}
			className={className}
			onDragOver={handleDragOver}
			onDrop={handleDrop}
			onDragLeave={handleDragLeave}
		>
			{children}
		</div>
	);
}
