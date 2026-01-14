"use client";

import { useCallback } from "react";
import { getComponentRenderer, type ComponentProps } from "../ComponentRegistry";
import type { ActionBinding, Style } from "../types";
import { useWidgetRefs } from "../WidgetRefsContext";

export interface WidgetInstanceComponentProps {
	widgetId: string;
	instanceId: string;
	appId?: string;
	exposedPropValues?: Record<string, unknown>;
	styleOverride?: Record<string, unknown>;
	actionBindings?: Record<string, ActionBinding>;
	style?: Style;
}

/**
 * A2UIWidgetInstance renders a widget instance by looking up the widget definition
 * from widgetRefs (stored on the page) and rendering its component tree.
 */
export function A2UIWidgetInstance({
	component,
	componentId,
	surfaceId,
	onAction,
}: ComponentProps) {
	const props = component as unknown as WidgetInstanceComponentProps;
	const { instanceId, widgetId } = props;
	const widgetRefsContext = useWidgetRefs();

	// Get widget definition from refs
	const widgetDef = widgetRefsContext?.getWidgetRef(instanceId);

	// Create a local renderChild for the widget's internal components
	const renderWidgetChild = useCallback(
		(childId: string, currentWidgetDef: typeof widgetDef): React.ReactNode => {
			if (!currentWidgetDef) return null;

			const childComponent = currentWidgetDef.components.find((c) => c.id === childId);
			if (!childComponent?.component) {
				console.warn(
					`Widget "${currentWidgetDef.name}" component "${childId}" not found. Available components:`,
					currentWidgetDef.components.map((c) => c.id),
				);
				return null;
			}

			const Renderer = getComponentRenderer(childComponent.component.type);
			if (!Renderer) {
				console.warn(`Unknown component type: ${childComponent.component.type}`);
				return null;
			}

			return (
				<Renderer
					key={childId}
					component={childComponent.component}
					componentId={childId}
					surfaceId={surfaceId}
					style={childComponent.style ?? childComponent.component.style}
					onAction={onAction}
					renderChild={(nestedChildId) => renderWidgetChild(nestedChildId, currentWidgetDef)}
				/>
			);
		},
		[surfaceId, onAction],
	);

	if (!widgetDef) {
		return (
			<div className="p-4 text-sm text-red-500 bg-red-50 rounded">
				Widget instance &quot;{instanceId}&quot; not found in refs
			</div>
		);
	}

	if (!widgetDef.rootComponentId) {
		return (
			<div className="p-4 text-sm text-red-500 bg-red-50 rounded">
				Widget definition missing rootComponentId
			</div>
		);
	}

	return (
		<div
			data-widget-instance={instanceId}
			data-widget-id={widgetId}
			className="contents"
		>
			{renderWidgetChild(widgetDef.rootComponentId, widgetDef)}
		</div>
	);
}

