// Builder exports
export {
	BuilderProvider,
	useBuilder,
	type BuilderContextType,
} from "./BuilderContext";
export { BuilderRenderer } from "./BuilderRenderer";
export { Canvas, type CanvasProps } from "./Canvas";
export {
	ComponentPalette,
	type ComponentPaletteProps,
} from "./ComponentPalette";
export { Inspector, type InspectorProps } from "./Inspector";
export { HierarchyTree, type HierarchyTreeProps } from "./HierarchyTree";
export {
	ResponsivePreview,
	SideBySidePreview,
	type ResponsivePreviewProps,
} from "./ResponsivePreview";
export { Toolbar, type ToolbarProps } from "./Toolbar";
export { CustomDragLayer } from "./CustomDragLayer";
export {
	WidgetBuilder,
	type WidgetBuilderProps,
	COMPONENT_DND_TYPE,
	COMPONENT_MOVE_TYPE,
	WIDGET_DND_TYPE,
	CONTAINER_TYPES,
	ROOT_ID,
	createDefaultComponent,
	type ComponentDragItem,
	type ComponentMoveItem,
	type WidgetDragItem,
} from "./WidgetBuilder";
export {
	WidgetSelector,
	type WidgetSelectorProps,
} from "./WidgetSelector";
export {
	SelectionManager,
	useMarqueeSelection,
	type BuilderSelectionRect,
	type SelectionManagerProps,
} from "./SelectionManager";
export {
	TransformHandles,
	type TransformBounds,
	type HandlePosition,
	type TransformHandlesProps,
} from "./TransformHandles";
export {
	SnapGuides,
	useSnapGuides,
	type SnapGuideLine,
	type ComponentBounds,
	type SnapGuidesProps,
} from "./SnapGuides";
export { Rulers, type RulersProps } from "./Rulers";
export {
	DragSource,
	DropTarget,
	type DragSourceProps,
	type DropTargetProps,
} from "./DragSource";
export {
	ComponentPreview,
	getComponentIcon,
	getComponentColors,
	type ComponentPreviewProps,
} from "./ComponentPreview";
export { FlowPilotAction, type FlowPilotActionProps } from "./FlowPilotAction";
export { A2UICopilot, type A2UICopilotProps } from "./a2ui-copilot";
export {
	WidgetInstanceInspector,
	type WidgetInstanceInspectorProps,
	type WorkflowEventOption,
} from "./WidgetInstanceInspector";
export { DevModePanel, type DevModePanelProps } from "./DevModePanel";
export {
	COMPONENT_SCHEMAS,
	getValidComponentTypes,
	isValidComponentType,
	getComponentSchema,
	getValidProperties,
	isValidProperty,
	type PropType,
	type PropSchema,
	type ComponentSchema,
} from "./componentSchema";
// Note: normalizeComponent, normalizeComponents, createDefaultComponent, getDefaultProps, getDefaultStyle
// are re-exported from "./WidgetBuilder"
