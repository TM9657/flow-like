"use client";

import { GripVertical, Trash2 } from "lucide-react";
import { Fragment, type ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import { useDrag, useDrop } from "react-dnd";
import { getEmptyImage } from "react-dnd-html5-backend";
import { cn } from "../../lib/utils";
import { useBackend } from "../../state/backend-state";
import type { IWidget } from "../../state/backend-state/widget-state";
import { ActionProvider } from "../a2ui/ActionHandler";
import {
	type ComponentProps,
	getComponentRenderer,
} from "../a2ui/ComponentRegistry";
import { DataProvider } from "../a2ui/DataContext";
import type {
	A2UIClientMessage,
	A2UIComponent,
	Children,
	Surface,
	SurfaceComponent,
} from "../a2ui/types";
import { WidgetRefsProvider } from "../a2ui/WidgetRefsContext";
import { useBuilder } from "./BuilderContext";
import {
	COMPONENT_DND_TYPE,
	COMPONENT_MOVE_TYPE,
	CONTAINER_TYPES,
	type ComponentDragItem,
	type ComponentMoveItem,
	ROOT_ID,
	WIDGET_DND_TYPE,
	type WidgetDragItem,
	createDefaultComponent,
	getDefaultStyle,
} from "./WidgetBuilder";

interface BuilderRendererProps {
	surface: Surface;
	className?: string;
}

export function BuilderRenderer({ surface, className }: BuilderRendererProps) {
	const { isDraggingGlobal, actionContext, widgetRefs } = useBuilder();
	const components = useMemo(
		() => surface.components ?? {},
		[surface.components],
	);

	const handleAction = useCallback((message: A2UIClientMessage) => {
		console.log("Builder action intercepted:", message);
	}, []);

	const renderComponent = useCallback(
		(componentId: string): ReactNode => {
			const surfaceComponent = components[componentId];
			if (!surfaceComponent) return null;

			return (
				<BuilderComponentWrapper
					key={componentId}
					componentId={componentId}
					surfaceComponent={surfaceComponent}
					surfaceId={surface.id}
					onAction={handleAction}
					renderChild={renderComponent}
					surfaceComponents={components}
				/>
			);
		},
		[components, surface.id, handleAction],
	);

	const rootComponent = surface.rootComponentId
		? components[surface.rootComponentId]
		: null;

	if (!rootComponent) {
		return (
			<div className={className}>
				<div className="text-muted-foreground text-sm">
					No content to display
				</div>
			</div>
		);
	}

	return (
		<DataProvider initialData={[]}>
			<WidgetRefsProvider widgetRefs={widgetRefs}>
				<ActionProvider
					onAction={handleAction}
					surfaceId={surface.id}
					appId={actionContext?.appId}
					components={components}
				>
					<div
						className={cn(className, isDraggingGlobal && "select-none")}
						style={{ userSelect: isDraggingGlobal ? "none" : undefined }}
					>
						{renderComponent(surface.rootComponentId)}
					</div>
				</ActionProvider>
			</WidgetRefsProvider>
		</DataProvider>
	);
}

// Drop zone between children for reordering
interface ChildDropZoneProps {
	parentId: string;
	index: number;
	isActive: boolean;
	onHover: (index: number) => void;
	onDrop: (
		item: ComponentDragItem | ComponentMoveItem | WidgetDragItem,
		index: number,
	) => void;
	isHorizontal?: boolean;
}

function ChildDropZone({
	parentId,
	index,
	isActive,
	onHover,
	onDrop,
	isHorizontal = false,
}: ChildDropZoneProps) {
	const [{ isOver }, drop] = useDrop<
		ComponentDragItem | ComponentMoveItem | WidgetDragItem,
		void,
		{ isOver: boolean }
	>(
		() => ({
			accept: [COMPONENT_DND_TYPE, COMPONENT_MOVE_TYPE, WIDGET_DND_TYPE],
			hover: () => {
				onHover(index);
			},
			drop: (item, monitor) => {
				if (monitor.didDrop()) return;
				onDrop(item, index);
			},
			collect: (monitor) => ({
				isOver: monitor.isOver({ shallow: true }),
			}),
		}),
		[index, onHover, onDrop],
	);

	const showIndicator = isActive || isOver;

	// Use absolute positioning overlay to avoid ANY layout shift
	return (
		<div
			ref={(node) => {
				if (node) drop(node);
			}}
			className="absolute pointer-events-auto"
			style={{
				// Position as a thin strip for drop detection
				...(isHorizontal
					? { top: 0, bottom: 0, width: 16, transform: "translateX(-50%)" }
					: { left: 0, right: 0, height: 16, transform: "translateY(-50%)" }),
				zIndex: 50,
			}}
		>
			{/* Visual indicator line */}
			<div
				className={cn(
					"absolute transition-all pointer-events-none",
					isHorizontal
						? "top-0 bottom-0 left-1/2 -translate-x-1/2"
						: "left-0 right-0 top-1/2 -translate-y-1/2",
					showIndicator ? "bg-primary" : "bg-transparent",
				)}
				style={{
					...(isHorizontal
						? { width: showIndicator ? 3 : 0 }
						: { height: showIndicator ? 3 : 0 }),
				}}
			/>
		</div>
	);
}

interface BuilderComponentWrapperProps {
	componentId: string;
	surfaceComponent: SurfaceComponent;
	surfaceId: string;
	onAction: (message: A2UIClientMessage) => void;
	renderChild: (childId: string) => ReactNode;
	/** Surface components map for presigned asset lookups */
	surfaceComponents: Record<string, SurfaceComponent>;
}

function BuilderComponentWrapper({
	componentId,
	surfaceComponent,
	surfaceId,
	onAction,
	renderChild,
	surfaceComponents,
}: BuilderComponentWrapperProps) {
	const { component, style } = surfaceComponent;
	const backend = useBackend();

	const {
		components,
		selection,
		selectComponent,
		addComponent,
		addComponents,
		updateComponent,
		deleteComponents,
		setIsDraggingGlobal,
		addWidgetRef,
		isComponentHidden,
	} = useBuilder();

	// Check hidden state early but don't return yet (hooks must be called unconditionally)
	const isHidden = component ? isComponentHidden(componentId) : false;

	const isContainer = component ? CONTAINER_TYPES.has(component.type) : false;
	const isRoot = componentId === ROOT_ID;
	const isSelected = selection.componentIds.includes(componentId);

	// Track which drop zone is being hovered
	const [hoverIndex, setHoverIndex] = useState<number | null>(null);

	const childrenData = component
		? ((component as unknown as Record<string, unknown>).children as Children | undefined)
		: undefined;
	const childIds =
		childrenData && "explicitList" in childrenData
			? childrenData.explicitList
			: [];

	// Find parent ID helper
	const findParentId = useCallback(
		(childId: string): string | null => {
			for (const [id, comp] of components) {
				if (!comp.component) continue;
				const compChildrenData = (
					comp.component as unknown as Record<string, unknown>
				).children as Children | undefined;
				const compChildren =
					compChildrenData && "explicitList" in compChildrenData
						? compChildrenData.explicitList
						: undefined;
				if (compChildren?.includes(childId)) {
					return id;
				}
			}
			return null;
		},
		[components],
	);

	// Check if targetId is a descendant of sourceId
	const isDescendant = useCallback(
		(targetId: string, sourceId: string): boolean => {
			const target = components.get(targetId);
			if (!target) return false;

			const targetChildrenData = (
				target.component as unknown as Record<string, unknown>
			).children as Children | undefined;
			const targetChildren =
				targetChildrenData && "explicitList" in targetChildrenData
					? targetChildrenData.explicitList
					: [];

			if (targetChildren.includes(sourceId)) return true;
			return targetChildren.some((cId) => isDescendant(cId, sourceId));
		},
		[components],
	);

	// Move component from one parent to another (with optional index for reordering)
	const moveComponent = useCallback(
		(
			movingId: string,
			fromParentId: string | null,
			toParentId: string,
			toIndex?: number,
		) => {
			// Get target parent
			const toParent = components.get(toParentId);
			if (!toParent) return;

			const toChildrenData = (
				toParent.component as unknown as Record<string, unknown>
			).children as Children | undefined;
			const toChildren =
				toChildrenData && "explicitList" in toChildrenData
					? [...toChildrenData.explicitList]
					: [];

			// If reordering within same parent
			if (fromParentId === toParentId) {
				const currentIndex = toChildren.indexOf(movingId);
				if (currentIndex === -1) return;

				// Remove from current position
				toChildren.splice(currentIndex, 1);

				// Insert at new position (adjust index if needed)
				const insertIndex =
					toIndex !== undefined
						? toIndex > currentIndex
							? toIndex - 1
							: toIndex
						: toChildren.length;
				toChildren.splice(insertIndex, 0, movingId);

				updateComponent(toParentId, {
					component: {
						...toParent.component,
						children: { explicitList: toChildren },
					} as A2UIComponent,
				});
			} else {
				// Moving to different parent
				// Remove from old parent
				if (fromParentId) {
					const fromParent = components.get(fromParentId);
					if (fromParent) {
						const fromChildrenData = (
							fromParent.component as unknown as Record<string, unknown>
						).children as Children | undefined;
						const fromChildren =
							fromChildrenData && "explicitList" in fromChildrenData
								? fromChildrenData.explicitList
								: [];
						updateComponent(fromParentId, {
							component: {
								...fromParent.component,
								children: {
									explicitList: fromChildren.filter((id) => id !== movingId),
								},
							} as A2UIComponent,
						});
					}
				}

				// Add to new parent at index
				const insertIndex = toIndex !== undefined ? toIndex : toChildren.length;
				toChildren.splice(insertIndex, 0, movingId);

				updateComponent(toParentId, {
					component: {
						...toParent.component,
						children: { explicitList: toChildren },
					} as A2UIComponent,
				});
			}
		},
		[components, updateComponent],
	);

	// Drag source for moving this component
	const [{ isDragging }, drag, dragPreview] = useDrag<
		ComponentMoveItem,
		void,
		{ isDragging: boolean }
	>(
		() => ({
			type: COMPONENT_MOVE_TYPE,
			item: () => {
				setIsDraggingGlobal(true);
				return {
					type: COMPONENT_MOVE_TYPE,
					componentId,
					currentParentId: findParentId(componentId),
				} as ComponentMoveItem;
			},
			canDrag: () => !isRoot,
			collect: (monitor) => ({
				isDragging: monitor.isDragging(),
			}),
			end: () => {
				setIsDraggingGlobal(false);
			},
		}),
		[componentId, isRoot, findParentId, setIsDraggingGlobal],
	);

	// Hide the default browser drag preview - use custom drag layer instead
	useEffect(() => {
		dragPreview(getEmptyImage(), { captureDraggingState: true });
	}, [dragPreview]);

	// Helper to insert a widget instance - stores widget definition in refs
	const insertWidgetInstance = useCallback(
		async (
			widgetItem: WidgetDragItem,
			parentId: string,
			insertIndex?: number,
		) => {
			const { appId, widgetId } = widgetItem;

			// Get parent BEFORE adding components to avoid stale closure
			const parent = components.get(parentId);
			if (!parent) return;

			// Fetch widget data
			let widget: IWidget;

			try {
				widget = await backend.widgetState.getWidget(appId, widgetId);
			} catch (err) {
				console.error("Failed to fetch widget:", err);
				return;
			}

			if (!widget.components?.length || !widget.rootComponentId) {
				console.warn("Widget has no components");
				return;
			}

			// Determine actual root component ID - prefer 'root', then stored value, then first component
			const componentIds = new Set(widget.components.map((c) => c.id));
			const effectiveRootId = componentIds.has("root")
				? "root"
				: componentIds.has(widget.rootComponentId)
					? widget.rootComponentId
					: widget.components[0]?.id ?? widget.rootComponentId;

			// Create a unique instance ID
			const instanceId = `widget-${widgetId}-${Date.now()}`;
			const widgetInstanceComponentId = `widgetInstance-${instanceId}`;

			// Store the widget definition in refs
			addWidgetRef(instanceId, {
				id: widget.id,
				name: widget.name,
				description: widget.description,
				rootComponentId: effectiveRootId,
				components: widget.components,
				dataModel: widget.dataModel,
				customizationOptions: widget.customizationOptions,
				exposedProps: widget.exposedProps,
				actions: widget.actions,
				tags: widget.tags ?? [],
				catalogId: widget.catalogId,
				thumbnail: widget.thumbnail,
				version: widget.version,
				createdAt: widget.createdAt,
				updatedAt: widget.updatedAt,
			});

			// Create a widgetInstance component that references the widget in refs
			const widgetInstanceComponent: SurfaceComponent = {
				id: widgetInstanceComponentId,
				component: {
					type: "widgetInstance",
					instanceId,
					widgetId,
					appId,
					exposedPropValues: {},
					actionBindings: {},
				} as A2UIComponent,
			};

			// Add the widget instance component
			addComponent(widgetInstanceComponent);

			// Add to parent's children (using captured parent)
			const parentChildren = (
				parent.component as unknown as Record<string, unknown>
			)?.children as Children | undefined;
			const existingChildren =
				parentChildren && "explicitList" in parentChildren
					? [...parentChildren.explicitList]
					: [];

			if (insertIndex !== undefined) {
				existingChildren.splice(insertIndex, 0, widgetInstanceComponentId);
			} else {
				existingChildren.push(widgetInstanceComponentId);
			}

			updateComponent(parentId, {
				component: {
					...parent.component,
					children: { explicitList: existingChildren },
				} as A2UIComponent,
			});
		},
		[backend.widgetState, components, addComponent, updateComponent, addWidgetRef],
	);

	// Drop target for receiving new components or moved components
	const [{ isOver, canDrop, isDraggingAnything }, drop] = useDrop<
		ComponentDragItem | ComponentMoveItem | WidgetDragItem,
		void,
		{ isOver: boolean; canDrop: boolean; isDraggingAnything: boolean }
	>(
		() => ({
			accept: [COMPONENT_DND_TYPE, COMPONENT_MOVE_TYPE, WIDGET_DND_TYPE],
			canDrop: (item) => {
				if (!isContainer) return false;
				if ("componentId" in item && item.componentId === componentId)
					return false;
				if (
					"componentId" in item &&
					isDescendant(componentId, item.componentId)
				)
					return false;
				return true;
			},
			hover: (item, monitor) => {
				if (!monitor.isOver({ shallow: true })) {
					setHoverIndex(null);
					return;
				}
				// For empty containers, always insert at index 0
				if (childIds.length === 0) {
					setHoverIndex(0);
					return;
				}
				// Default to end if not over a specific child
				setHoverIndex(childIds.length);
			},
			drop: (item, monitor) => {
				if (monitor.didDrop()) return;
				const insertIndex = hoverIndex ?? childIds.length;

				if ("widgetId" in item) {
					// Handle widget drop - fire and forget async operation
					insertWidgetInstance(
						item as WidgetDragItem,
						componentId,
						insertIndex,
					).then(() => {
						setHoverIndex(null);
					});
					return;
				}

				if ("componentType" in item) {
					const newId = `${item.componentType}-${Date.now()}`;
					const defaultStyle = getDefaultStyle(item.componentType);
					const newComponent: SurfaceComponent = {
						id: newId,
						component: createDefaultComponent(item.componentType),
						...(defaultStyle && { style: defaultStyle }),
					};
					addComponent(newComponent);

					// Insert at specific index
					const newChildren = [...childIds];
					newChildren.splice(insertIndex, 0, newId);
					updateComponent(componentId, {
						component: {
							...component,
							children: { explicitList: newChildren },
						} as A2UIComponent,
					});
				} else if ("componentId" in item) {
					moveComponent(
						item.componentId,
						item.currentParentId,
						componentId,
						insertIndex,
					);
				}
				setHoverIndex(null);
			},
			collect: (monitor) => ({
				isOver: monitor.isOver({ shallow: true }),
				canDrop: monitor.canDrop(),
				isDraggingAnything:
					monitor.getItemType() === COMPONENT_DND_TYPE ||
					monitor.getItemType() === COMPONENT_MOVE_TYPE ||
					monitor.getItemType() === WIDGET_DND_TYPE,
			}),
		}),
		[
			componentId,
			component,
			childIds,
			isContainer,
			isDescendant,
			addComponent,
			updateComponent,
			moveComponent,
			insertWidgetInstance,
			hoverIndex,
		],
	);

	const handleClick = useCallback(
		(e: React.MouseEvent) => {
			e.preventDefault();
			e.stopPropagation();
			selectComponent(componentId, e.shiftKey || e.metaKey);
		},
		[componentId, selectComponent],
	);

	const handleDelete = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation();
			if (isRoot) return;

			const parentId = findParentId(componentId);
			if (parentId) {
				const parent = components.get(parentId);
				if (parent) {
					const parentChildrenData = (
						parent.component as unknown as Record<string, unknown>
					).children as Children | undefined;
					const parentChildren =
						parentChildrenData && "explicitList" in parentChildrenData
							? parentChildrenData.explicitList
							: [];
					updateComponent(parentId, {
						component: {
							...parent.component,
							children: {
								explicitList: parentChildren.filter(
									(cid: string) => cid !== componentId,
								),
							},
						} as A2UIComponent,
					});
				}
			}
			deleteComponents([componentId]);
		},
		[
			componentId,
			components,
			updateComponent,
			deleteComponents,
			isRoot,
			findParentId,
		],
	);

	// Handle drop at specific index
	const handleDropAtIndex = useCallback(
		(
			item: ComponentDragItem | ComponentMoveItem | WidgetDragItem,
			index: number,
		) => {
			if ("widgetId" in item) {
				// Handle widget drop - fire and forget async operation
				insertWidgetInstance(item as WidgetDragItem, componentId, index).then(
					() => {
						setHoverIndex(null);
					},
				);
				return;
			}

			if ("componentType" in item) {
				const newId = `${item.componentType}-${Date.now()}`;
				const defaultStyle = getDefaultStyle(item.componentType);
				const newComponent: SurfaceComponent = {
					id: newId,
					component: createDefaultComponent(item.componentType),
					...(defaultStyle && { style: defaultStyle }),
				};
				addComponent(newComponent);

				const newChildren = [...childIds];
				newChildren.splice(index, 0, newId);
				updateComponent(componentId, {
					component: {
						...component,
						children: { explicitList: newChildren },
					} as A2UIComponent,
				});
			} else if ("componentId" in item) {
				moveComponent(
					item.componentId,
					item.currentParentId,
					componentId,
					index,
				);
			}
			setHoverIndex(null);
		},
		[
			componentId,
			component,
			childIds,
			addComponent,
			updateComponent,
			moveComponent,
			insertWidgetInstance,
		],
	);

	// Determine if this is a horizontal layout (row)
	const isHorizontalLayout = component.type === "row";

	// Custom renderChild that adds drop zones without breaking flex/grid layout
	const renderChildWithDropZones = useCallback(
		(childId: string, index: number): ReactNode => {
			// Use surfaceComponents for presigned assets, fall back to builder context
			const childComponent = surfaceComponents[childId] ?? components.get(childId);
			if (!childComponent) return null;

			return (
				<Fragment key={childId}>
					{/* Drop zone positioned before this element - only show when dragging */}
					{isDraggingAnything && (
						<div
							className="contents"
						>
							<div
								className="absolute pointer-events-none"
								style={{
									...(isHorizontalLayout
										? { left: 0, top: 0, bottom: 0, width: 0 }
										: { top: 0, left: 0, right: 0, height: 0 }),
									zIndex: 50,
								}}
							>
								<ChildDropZone
									parentId={componentId}
									index={index}
									isActive={hoverIndex === index}
									onHover={setHoverIndex}
									onDrop={handleDropAtIndex}
									isHorizontal={isHorizontalLayout}
								/>
							</div>
						</div>
					)}
					<BuilderComponentWrapper
						componentId={childId}
						surfaceComponent={childComponent}
						surfaceId={surfaceId}
						onAction={onAction}
						renderChild={renderChild}
						surfaceComponents={surfaceComponents}
					/>
				</Fragment>
			);
		},
		[
			components,
			surfaceComponents,
			componentId,
			surfaceId,
			onAction,
			renderChild,
			isDraggingAnything,
			hoverIndex,
			handleDropAtIndex,
			isHorizontalLayout,
		],
	);

	// Modified props that render children with drop zones
	const modifiedRenderChild = useCallback(
		(childId: string): ReactNode => {
			const childIndex = childIds.indexOf(childId);
			if (childIndex === -1) return renderChild(childId);
			return renderChildWithDropZones(childId, childIndex);
		},
		[childIds, renderChild, renderChildWithDropZones],
	);

	// Get the renderer (must be after all hooks)
	const Renderer = component ? getComponentRenderer(component.type) : null;

	// Return null for invalid states (AFTER all hooks)
	if (!component || !Renderer) {
		if (component && !Renderer) {
			console.warn(`Unknown component type: ${component.type}`);
		}
		return null;
	}

	// If component is hidden in builder, don't render it
	if (isHidden) {
		return null;
	}

	const props: ComponentProps = {
		component,
		componentId,
		surfaceId,
		style: style ?? component.style,
		onAction,
		renderChild: modifiedRenderChild,
	};

	const showContainerHighlight = isContainer && isOver && canDrop;
	const showDropIndicator = isOver && canDrop && childIds.length === 0;

	// Determine if this component needs a wrapper that affects layout
	// Root and containers need the wrapper for drop targets
	// Other components use display:contents to preserve parent flex/grid layout
	const needsLayoutWrapper = isRoot || isContainer || isSelected || showContainerHighlight || showDropIndicator;

	return (
		<div
			ref={(node) => {
				if (isContainer) drop(node);
			}}
			onClick={handleClick}
			onKeyDown={(e) => e.key === "Enter" && handleClick(e as never)}
			className={cn(
				"group [&_input]:pointer-events-none [&_button]:pointer-events-none [&_textarea]:pointer-events-none [&_select]:pointer-events-none [&_a]:pointer-events-none",
				// Use display:contents when not selected and not a container to preserve flex/grid layout
				!needsLayoutWrapper && "contents",
				needsLayoutWrapper && "relative",
				showContainerHighlight && "ring-2 ring-dashed ring-primary/50",
				showContainerHighlight && childIds.length === 0 && "min-h-[60px]",
				showDropIndicator && "ring-2 ring-primary bg-primary/10",
				isSelected && !isRoot && "ring-2 ring-blue-500",
				isDragging && "opacity-50",
			)}
			data-builder-component={componentId}
		>
			{isSelected && !isRoot && (
				<div className="absolute -top-7 left-0 z-20 flex items-center gap-0.5 px-1.5 py-0.5 bg-blue-500 text-white rounded text-xs shadow pointer-events-auto">
					<div
						ref={(el) => {
							if (el) drag(el);
						}}
						className="cursor-grab p-0.5 hover:bg-white/20 rounded"
					>
						<GripVertical className="h-3 w-3" />
					</div>
					<span className="capitalize font-medium px-1">{component.type}</span>
					<button
						type="button"
						onClick={handleDelete}
						className="p-0.5 hover:bg-white/20 rounded"
					>
						<Trash2 className="h-3 w-3" />
					</button>
				</div>
			)}

			{!isSelected && !isRoot && needsLayoutWrapper && (
				<div className="absolute inset-0 pointer-events-none border border-transparent group-hover:border-muted-foreground/30 rounded transition-colors z-10" />
			)}

			{showDropIndicator && (
				<div className="absolute inset-0 flex items-center justify-center border-2 border-dashed border-primary rounded pointer-events-none z-10">
					<span className="text-xs text-primary font-medium">Drop here</span>
				</div>
			)}

			<Renderer {...props} />

			{/* Final drop zone after all children - positioned at container end */}
			{isContainer && isDraggingAnything && childIds.length > 0 && (
				<div
					className="absolute pointer-events-none"
					style={{
						...(isHorizontalLayout
							? { right: 0, top: 0, bottom: 0, width: 0 }
							: { bottom: 0, left: 0, right: 0, height: 0 }),
						zIndex: 50,
					}}
				>
					<ChildDropZone
						parentId={componentId}
						index={childIds.length}
						isActive={hoverIndex === childIds.length}
						onHover={setHoverIndex}
						onDrop={handleDropAtIndex}
						isHorizontal={isHorizontalLayout}
					/>
				</div>
			)}
		</div>
	);
}
