"use client";

import { useDraggable } from "@dnd-kit/core";
import {
	AlignCenter,
	Calendar,
	CheckSquare,
	ChevronRight,
	Circle,
	Columns3,
	CreditCard,
	Image,
	ImagePlus,
	Layers,
	LayoutGrid,
	Link2,
	List,
	Loader2,
	MessageSquare,
	MousePointer,
	PanelLeft,
	Rows3,
	Search,
	SlidersHorizontal,
	Space,
	Square,
	Star,
	Table2,
	ToggleLeft,
	Type,
	Upload,
	Video,
} from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useInvoke } from "../../hooks";
import { cn } from "../../lib";
import { useBackend } from "../../state/backend-state";
import type { IUserWidgetInfo } from "../../state/backend-state/user-state";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../ui/collapsible";
import { Input } from "../ui/input";
import { ScrollArea } from "../ui/scroll-area";
import { useBuilder } from "./BuilderContext";
import {
	COMPONENT_DND_TYPE,
	type ComponentDragData,
	WIDGET_DND_TYPE,
	type WidgetDragData,
} from "./BuilderDndContext";
import { getDefaultProps } from "./componentDefaults";

interface ComponentDefinition {
	type: string;
	label: string;
	icon: typeof Columns3;
	category: string;
	description?: string;
}

const COMPONENT_DEFINITIONS: ComponentDefinition[] = [
	// Layout
	{
		type: "row",
		label: "Row",
		icon: Columns3,
		category: "Layout",
		description: "Horizontal flex container",
	},
	{
		type: "column",
		label: "Column",
		icon: Rows3,
		category: "Layout",
		description: "Vertical flex container",
	},
	{
		type: "stack",
		label: "Stack",
		icon: Layers,
		category: "Layout",
		description: "Z-axis layering",
	},
	{
		type: "grid",
		label: "Grid",
		icon: LayoutGrid,
		category: "Layout",
		description: "CSS Grid layout",
	},
	{
		type: "scrollArea",
		label: "Scroll Area",
		icon: PanelLeft,
		category: "Layout",
		description: "Scrollable container",
	},
	{
		type: "aspectRatio",
		label: "Aspect Ratio",
		icon: Square,
		category: "Layout",
		description: "Maintain aspect ratio",
	},
	{
		type: "overlay",
		label: "Overlay",
		icon: Layers,
		category: "Layout",
		description: "Positioned overlays",
	},
	{
		type: "absolute",
		label: "Absolute",
		icon: Square,
		category: "Layout",
		description: "Free positioning",
	},
	{
		type: "box",
		label: "Box",
		icon: Square,
		category: "Layout",
		description: "Generic container with semantic HTML",
	},
	{
		type: "center",
		label: "Center",
		icon: AlignCenter,
		category: "Layout",
		description: "Centers content horizontally and vertically",
	},
	{
		type: "spacer",
		label: "Spacer",
		icon: Space,
		category: "Layout",
		description: "Flexible or fixed space between elements",
	},

	// Display
	{
		type: "text",
		label: "Text",
		icon: Type,
		category: "Display",
		description: "Text content",
	},
	{
		type: "image",
		label: "Image",
		icon: Image,
		category: "Display",
		description: "Image from URL",
	},
	{
		type: "video",
		label: "Video",
		icon: Video,
		category: "Display",
		description: "Video player",
	},
	{
		type: "icon",
		label: "Icon",
		icon: Star,
		category: "Display",
		description: "Icon from set",
	},
	{
		type: "markdown",
		label: "Markdown",
		icon: MessageSquare,
		category: "Display",
		description: "Markdown content",
	},
	{
		type: "divider",
		label: "Divider",
		icon: Square,
		category: "Display",
		description: "Visual separator",
	},
	{
		type: "badge",
		label: "Badge",
		icon: Square,
		category: "Display",
		description: "Status badge",
	},
	{
		type: "avatar",
		label: "Avatar",
		icon: Circle,
		category: "Display",
		description: "User avatar",
	},
	{
		type: "progress",
		label: "Progress",
		icon: SlidersHorizontal,
		category: "Display",
		description: "Progress bar",
	},
	{
		type: "spinner",
		label: "Spinner",
		icon: Circle,
		category: "Display",
		description: "Loading spinner",
	},
	{
		type: "skeleton",
		label: "Skeleton",
		icon: Square,
		category: "Display",
		description: "Loading skeleton",
	},
	{
		type: "iframe",
		label: "IFrame",
		icon: Square,
		category: "Display",
		description: "Embedded webpage",
	},
	{
		type: "plotlyChart",
		label: "Plotly Chart",
		icon: SlidersHorizontal,
		category: "Display",
		description: "Interactive chart",
	},
	{
		type: "table",
		label: "Table",
		icon: Table2,
		category: "Display",
		description: "Data table with sorting and filtering",
	},
	{
		type: "lottie",
		label: "Lottie",
		icon: Video,
		category: "Display",
		description: "Lottie animation",
	},
	{
		type: "filePreview",
		label: "File Preview",
		icon: Image,
		category: "Display",
		description: "Preview files (PDF, images, etc)",
	},
	{
		type: "nivoChart",
		label: "Nivo Chart",
		icon: SlidersHorizontal,
		category: "Display",
		description: "Nivo data visualization",
	},
	{
		type: "boundingBoxOverlay",
		label: "Bounding Box",
		icon: Square,
		category: "Display",
		description: "Draw bounding boxes over images",
	},

	// Interactive
	{
		type: "button",
		label: "Button",
		icon: MousePointer,
		category: "Interactive",
		description: "Clickable button",
	},
	{
		type: "textField",
		label: "Text Field",
		icon: Type,
		category: "Interactive",
		description: "Text input",
	},
	{
		type: "select",
		label: "Select",
		icon: List,
		category: "Interactive",
		description: "Dropdown select",
	},
	{
		type: "slider",
		label: "Slider",
		icon: SlidersHorizontal,
		category: "Interactive",
		description: "Range slider",
	},
	{
		type: "checkbox",
		label: "Checkbox",
		icon: CheckSquare,
		category: "Interactive",
		description: "Boolean checkbox",
	},
	{
		type: "switch",
		label: "Switch",
		icon: ToggleLeft,
		category: "Interactive",
		description: "Toggle switch",
	},
	{
		type: "radioGroup",
		label: "Radio Group",
		icon: Circle,
		category: "Interactive",
		description: "Radio options",
	},
	{
		type: "dateTimeInput",
		label: "Date/Time",
		icon: Calendar,
		category: "Interactive",
		description: "Date/time picker",
	},
	{
		type: "fileInput",
		label: "File Input",
		icon: Upload,
		category: "Interactive",
		description: "File upload",
	},
	{
		type: "imageInput",
		label: "Image Input",
		icon: ImagePlus,
		category: "Interactive",
		description: "Image upload with preview",
	},
	{
		type: "link",
		label: "Link",
		icon: Link2,
		category: "Interactive",
		description: "Anchor link",
	},
	{
		type: "imageLabeler",
		label: "Image Labeler",
		icon: Image,
		category: "Interactive",
		description: "Draw and label regions on images",
	},
	{
		type: "imageHotspot",
		label: "Image Hotspot",
		icon: MousePointer,
		category: "Interactive",
		description: "Clickable hotspots on images",
	},

	// Container
	{
		type: "card",
		label: "Card",
		icon: CreditCard,
		category: "Container",
		description: "Card container",
	},
	{
		type: "modal",
		label: "Modal",
		icon: Square,
		category: "Container",
		description: "Modal dialog",
	},
	{
		type: "tabs",
		label: "Tabs",
		icon: Columns3,
		category: "Container",
		description: "Tab container",
	},
	{
		type: "accordion",
		label: "Accordion",
		icon: Rows3,
		category: "Container",
		description: "Collapsible sections",
	},
	{
		type: "drawer",
		label: "Drawer",
		icon: PanelLeft,
		category: "Container",
		description: "Slide-out panel",
	},
	{
		type: "tooltip",
		label: "Tooltip",
		icon: MessageSquare,
		category: "Container",
		description: "Hover tooltip",
	},
	{
		type: "popover",
		label: "Popover",
		icon: MessageSquare,
		category: "Container",
		description: "Click popover",
	},

	// Game
	{
		type: "canvas2d",
		label: "Canvas 2D",
		icon: Square,
		category: "Game",
		description: "2D game canvas",
	},
	{
		type: "sprite",
		label: "Sprite",
		icon: Image,
		category: "Game",
		description: "2D sprite",
	},
	{
		type: "shape",
		label: "Shape",
		icon: Square,
		category: "Game",
		description: "2D shape primitive",
	},
	{
		type: "scene3d",
		label: "Scene 3D",
		icon: Square,
		category: "Game",
		description: "3D scene",
	},
	{
		type: "model3d",
		label: "Model 3D",
		icon: Square,
		category: "Game",
		description: "3D model",
	},
	{
		type: "dialogue",
		label: "Dialogue",
		icon: MessageSquare,
		category: "Game",
		description: "Visual novel dialogue",
	},
	{
		type: "characterPortrait",
		label: "Portrait",
		icon: Image,
		category: "Game",
		description: "Character portrait",
	},
	{
		type: "choiceMenu",
		label: "Choice Menu",
		icon: List,
		category: "Game",
		description: "Branching choices",
	},
	{
		type: "inventoryGrid",
		label: "Inventory",
		icon: LayoutGrid,
		category: "Game",
		description: "Game inventory",
	},
	{
		type: "healthBar",
		label: "Health Bar",
		icon: SlidersHorizontal,
		category: "Game",
		description: "Stat bar",
	},
	{
		type: "miniMap",
		label: "Mini Map",
		icon: Square,
		category: "Game",
		description: "Game minimap",
	},
];

const CATEGORIES = ["Layout", "Display", "Interactive", "Container", "Game"];

export interface ComponentPaletteProps {
	className?: string;
	onDragStart?: (type: string) => void;
	onWidgetDragStart?: (appId: string, widgetId: string) => void;
	currentAppId?: string;
	showWidgets?: boolean;
}

export function ComponentPalette({
	className,
	onDragStart,
	onWidgetDragStart,
	currentAppId,
	showWidgets = true,
}: ComponentPaletteProps) {
	const backend = useBackend();
	const [searchQuery, setSearchQuery] = useState("");
	const [openCategories, setOpenCategories] = useState<Set<string>>(
		new Set(["Layout", "Display", "Interactive"]),
	);
	const [recentlyUsed, setRecentlyUsed] = useState<string[]>([]);
	const [widgetsSectionOpen, setWidgetsSectionOpen] = useState(true);

	const { addComponent } = useBuilder();

	const { data: widgets, isLoading: widgetsLoading } = useInvoke(
		backend.userState.getUserWidgets,
		backend.userState,
		[],
	);

	const filteredComponents = useMemo(() => {
		if (!searchQuery.trim()) return COMPONENT_DEFINITIONS;
		const query = searchQuery.toLowerCase();
		return COMPONENT_DEFINITIONS.filter(
			(c) =>
				c.label.toLowerCase().includes(query) ||
				c.type.toLowerCase().includes(query) ||
				c.category.toLowerCase().includes(query) ||
				c.description?.toLowerCase().includes(query),
		);
	}, [searchQuery]);

	const filteredWidgets = useMemo(() => {
		if (!widgets || !showWidgets) return [];
		if (!searchQuery.trim()) return widgets;
		const query = searchQuery.toLowerCase();
		return widgets.filter(
			(w) =>
				w.metadata.name.toLowerCase().includes(query) ||
				w.widgetId.toLowerCase().includes(query) ||
				w.metadata.description?.toLowerCase().includes(query) ||
				w.metadata.tags?.some((tag) => tag.toLowerCase().includes(query)),
		);
	}, [widgets, searchQuery, showWidgets]);

	const groupedComponents = useMemo(() => {
		const groups: Record<string, ComponentDefinition[]> = {};
		for (const category of CATEGORIES) {
			groups[category] = filteredComponents.filter(
				(c) => c.category === category,
			);
		}
		return groups;
	}, [filteredComponents]);

	const toggleCategory = useCallback((category: string) => {
		setOpenCategories((prev) => {
			const next = new Set(prev);
			if (next.has(category)) {
				next.delete(category);
			} else {
				next.add(category);
			}
			return next;
		});
	}, []);

	const trackRecentlyUsed = useCallback(
		(type: string) => {
			setRecentlyUsed((prev) => {
				const next = [type, ...prev.filter((t) => t !== type)].slice(0, 5);
				return next;
			});
			onDragStart?.(type);
		},
		[onDragStart],
	);

	const handleDoubleClick = useCallback(
		(type: string) => {
			// Quick-add to canvas center
			const component = {
				id: `${type}-${Date.now()}`,
				type,
				component: { type, ...getDefaultProps(type) } as never,
			};
			addComponent(component);
		},
		[addComponent],
	);

	return (
		<div
			className={cn(
				"flex flex-col h-full bg-background border-r overflow-hidden",
				className,
			)}
		>
			<div className="p-3 border-b shrink-0">
				<div className="relative">
					<Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
					<Input
						placeholder="Search components..."
						value={searchQuery}
						onChange={(e) => setSearchQuery(e.target.value)}
						className="pl-8"
					/>
				</div>
			</div>

			<ScrollArea className="flex-1 min-h-0">
				<div className="p-2 space-y-1">
					{/* Recently used */}
					{recentlyUsed.length > 0 && !searchQuery && (
						<Collapsible defaultOpen>
							<CollapsibleTrigger className="flex w-full items-center justify-between p-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded">
								<span>Recent</span>
								<ChevronRight className="h-4 w-4 transition-transform duration-200 data-[state=open]:rotate-90" />
							</CollapsibleTrigger>
							<CollapsibleContent className="pt-1 space-y-0.5">
								{recentlyUsed.map((type) => {
									const def = COMPONENT_DEFINITIONS.find(
										(c) => c.type === type,
									);
									if (!def) return null;
									return (
										<ComponentItem
											key={`recent-${type}`}
											definition={def}
											onUse={trackRecentlyUsed}
											onDoubleClick={handleDoubleClick}
										/>
									);
								})}
							</CollapsibleContent>
						</Collapsible>
					)}

					{/* Categories */}
					{CATEGORIES.map((category) => {
						const components = groupedComponents[category];
						if (components.length === 0) return null;

						return (
							<Collapsible
								key={category}
								open={openCategories.has(category)}
								onOpenChange={() => toggleCategory(category)}
							>
								<CollapsibleTrigger className="flex w-full items-center justify-between p-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded">
									<span>{category}</span>
									<ChevronRight
										className={cn(
											"h-4 w-4 transition-transform duration-200",
											openCategories.has(category) && "rotate-90",
										)}
									/>
								</CollapsibleTrigger>
								<CollapsibleContent className="pt-1 space-y-0.5">
									{components.map((def) => (
										<ComponentItem
											key={def.type}
											definition={def}
											onUse={trackRecentlyUsed}
											onDoubleClick={handleDoubleClick}
										/>
									))}
								</CollapsibleContent>
							</Collapsible>
						);
					})}

					{/* Widgets section */}
					{showWidgets && (
						<Collapsible
							open={widgetsSectionOpen}
							onOpenChange={setWidgetsSectionOpen}
						>
							<CollapsibleTrigger className="flex w-full items-center justify-between p-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded">
								<div className="flex items-center gap-2">
									<Layers className="h-4 w-4" />
									<span>Widgets</span>
								</div>
								<ChevronRight
									className={cn(
										"h-4 w-4 transition-transform duration-200",
										widgetsSectionOpen && "rotate-90",
									)}
								/>
							</CollapsibleTrigger>
							<CollapsibleContent className="pt-1 space-y-0.5">
								{widgetsLoading ? (
									<div className="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground">
										<Loader2 className="h-4 w-4 animate-spin" />
										<span>Loading widgets...</span>
									</div>
								) : filteredWidgets.length === 0 ? (
									<div className="px-3 py-2 text-sm text-muted-foreground">
										{searchQuery ? "No widgets match" : "No widgets available"}
									</div>
								) : (
									filteredWidgets.map((widget) => (
										<WidgetItem
											key={`${widget.appId}-${widget.widgetId}`}
											widget={widget}
											onDragStart={onWidgetDragStart}
										/>
									))
								)}
							</CollapsibleContent>
						</Collapsible>
					)}
				</div>
			</ScrollArea>
		</div>
	);
}

interface ComponentItemProps {
	definition: ComponentDefinition;
	onUse: (type: string) => void;
	onDoubleClick: (type: string) => void;
}

function ComponentItem({
	definition,
	onUse,
	onDoubleClick,
}: ComponentItemProps) {
	const Icon = definition.icon;

	const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
		id: `palette-${definition.type}`,
		data: {
			type: COMPONENT_DND_TYPE,
			componentType: definition.type,
		} satisfies ComponentDragData,
	});

	return (
		<div
			ref={setNodeRef}
			{...listeners}
			{...attributes}
			onDoubleClick={() => onDoubleClick(definition.type)}
			className={cn(
				"flex items-center gap-2 px-3 py-2 text-sm rounded cursor-grab hover:bg-muted active:cursor-grabbing select-none touch-none",
				isDragging && "opacity-50",
			)}
			title={definition.description}
		>
			<Icon className="h-4 w-4 text-muted-foreground shrink-0" />
			<span className="truncate">{definition.label}</span>
		</div>
	);
}

interface WidgetItemProps {
	widget: IUserWidgetInfo;
	onDragStart?: (appId: string, widgetId: string) => void;
}

function WidgetItem({ widget, onDragStart }: WidgetItemProps) {
	const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
		id: `widget-${widget.appId}-${widget.widgetId}`,
		data: {
			type: WIDGET_DND_TYPE,
			appId: widget.appId,
			widgetId: widget.widgetId,
		} satisfies WidgetDragData,
	});

	return (
		<div
			ref={setNodeRef}
			{...listeners}
			{...attributes}
			className={cn(
				"flex items-center gap-2 px-3 py-2 text-sm rounded cursor-grab hover:bg-muted active:cursor-grabbing select-none touch-none",
				isDragging && "opacity-50",
			)}
			title={widget.metadata.description}
		>
			{widget.metadata.thumbnail ? (
				<img
					src={widget.metadata.thumbnail}
					alt=""
					className="h-4 w-4 rounded object-cover shrink-0"
				/>
			) : (
				<Layers className="h-4 w-4 text-muted-foreground shrink-0" />
			)}
			<span className="truncate">{widget.metadata.name}</span>
		</div>
	);
}
