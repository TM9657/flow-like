"use client";

import { Layers, type LucideIcon } from "lucide-react";
import { memo } from "react";
import { useDragLayer } from "react-dnd";
import { cn } from "../../lib/utils";
import {
	COMPONENT_DND_TYPE,
	COMPONENT_MOVE_TYPE,
	WIDGET_DND_TYPE,
	type ComponentDragItem,
	type ComponentMoveItem,
	type WidgetDragItem,
} from "./WidgetBuilder";

// Component type to icon mapping
const COMPONENT_ICONS: Record<string, string> = {
	row: "⬌",
	column: "⬍",
	grid: "⊞",
	stack: "☰",
	text: "T",
	button: "☐",
	image: "🖼",
	card: "▢",
	textField: "⌨",
	select: "▼",
	checkbox: "☑",
	switch: "◐",
};

interface DragLayerProps {
	className?: string;
}

export const CustomDragLayer = memo(function CustomDragLayer({
	className,
}: DragLayerProps) {
	const { itemType, isDragging, item, currentOffset, initialOffset } = useDragLayer(
		(monitor) => ({
			item: monitor.getItem() as ComponentDragItem | ComponentMoveItem | WidgetDragItem | null,
			itemType: monitor.getItemType(),
			isDragging: monitor.isDragging(),
			initialOffset: monitor.getInitialSourceClientOffset(),
			currentOffset: monitor.getClientOffset(),
		}),
	);

	if (!isDragging || !currentOffset || !item) {
		return null;
	}

	const getLabel = (): string => {
		if (!item) return "Component";

		if ("componentType" in item) {
			return item.componentType;
		}
		if ("componentId" in item) {
			// Extract type from ID (e.g., "button-12345" -> "button")
			const match = item.componentId.match(/^([a-zA-Z]+)/);
			return match?.[1] || "Component";
		}
		if ("widgetId" in item) {
			return "Widget";
		}
		return "Component";
	};

	const getIcon = (): string => {
		if (!item) return "▢";

		if ("componentType" in item) {
			return COMPONENT_ICONS[item.componentType] || "▢";
		}
		if ("componentId" in item) {
			const match = item.componentId.match(/^([a-zA-Z]+)/);
			const type = match?.[1] || "";
			return COMPONENT_ICONS[type] || "▢";
		}
		if ("widgetId" in item) {
			return "☰";
		}
		return "▢";
	};

	const label = getLabel();
	const icon = getIcon();
	const isMove = itemType === COMPONENT_MOVE_TYPE;

	return (
		<div
			className={cn("fixed inset-0 pointer-events-none z-10000", className)}
			style={{ left: 0, top: 0 }}
		>
			<div
				className={cn(
					"inline-flex items-center gap-2 px-3 py-1.5 rounded-lg shadow-lg text-sm font-medium",
					isMove
						? "bg-blue-500 text-white"
						: "bg-primary text-primary-foreground",
				)}
				style={{
					transform: `translate(${currentOffset.x + 16}px, ${currentOffset.y + 16}px)`,
					willChange: "transform",
				}}
			>
				<span className="text-base">{icon}</span>
				<span className="capitalize">{label}</span>
			</div>
		</div>
	);
});
