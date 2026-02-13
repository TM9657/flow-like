"use client";

import { useCallback, useMemo } from "react";
import { ActionProvider } from "./ActionHandler";
import { type ComponentProps, getComponentRenderer } from "./ComponentRegistry";
import { DataProvider } from "./DataContext";
import { type IWidgetRef, WidgetRefsProvider } from "./WidgetRefsContext";
import type {
	A2UIClientMessage,
	A2UIServerMessage,
	Surface,
	SurfaceComponent,
} from "./types";

export interface A2UIRendererProps {
	surface: Surface;
	widgetRefs?: Record<string, IWidgetRef>;
	onMessage?: (message: A2UIClientMessage) => void;
	onA2UIMessage?: (message: A2UIServerMessage) => void;
	className?: string;
	appId?: string;
	boardId?: string;
	isPreviewMode?: boolean;
	openDialog?: (
		route: string,
		title?: string,
		queryParams?: Record<string, string>,
		dialogId?: string,
	) => void;
	closeDialog?: (dialogId?: string) => void;
}

export function A2UIRenderer({
	surface,
	widgetRefs,
	onMessage,
	onA2UIMessage,
	className,
	appId,
	boardId,
	isPreviewMode = false,
	openDialog,
	closeDialog,
}: A2UIRendererProps) {
	const components = useMemo(
		() => surface.components ?? {},
		[surface.components],
	);

	const handleAction = useCallback(
		(message: A2UIClientMessage) => {
			onMessage?.(message);
		},
		[onMessage],
	);

	const renderComponent = useCallback(
		(componentId: string): React.ReactNode => {
			const surfaceComponent = components[componentId];
			if (!surfaceComponent?.component) return null;

			const { component, style } = surfaceComponent;
			const Renderer = getComponentRenderer(component.type);
			if (!Renderer) {
				console.warn(`Unknown component type: ${component.type}`);
				return null;
			}

			const props: ComponentProps = {
				component,
				componentId,
				surfaceId: surface.id,
				style: style ?? component.style,
				onAction: handleAction,
				renderChild: renderComponent,
			};

			return <Renderer key={componentId} {...props} />;
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
					onA2UIMessage={onA2UIMessage}
					surfaceId={surface.id}
					appId={appId}
					boardId={boardId}
					components={components}
					isPreviewMode={isPreviewMode}
					openDialog={openDialog}
					closeDialog={closeDialog}
				>
					<div className={className}>
						{renderComponent(surface.rootComponentId)}
					</div>
				</ActionProvider>
			</WidgetRefsProvider>
		</DataProvider>
	);
}

export interface A2UIMessageHandlerProps {
	onServerMessage: (message: A2UIServerMessage) => void;
	children: (props: {
		surfaces: Map<string, Surface>;
		sendMessage: (msg: A2UIClientMessage) => void;
	}) => React.ReactNode;
}

export function useA2UIState() {
	const applyServerMessage = useCallback(
		(
			surfaces: Map<string, Surface>,
			message: A2UIServerMessage,
		): Map<string, Surface> => {
			const newSurfaces = new Map(surfaces);

			if (message.type === "beginRendering") {
				const componentsMap: Record<string, SurfaceComponent> = {};
				for (const comp of message.components) {
					componentsMap[comp.id] = comp;
				}
				newSurfaces.set(message.surfaceId, {
					id: message.surfaceId,
					rootComponentId: message.rootComponentId,
					components: componentsMap,
					catalogId: message.catalogId,
				});
			}

			if (message.type === "surfaceUpdate") {
				const existing = newSurfaces.get(message.surfaceId);
				if (existing) {
					const updatedComponents = { ...existing.components };
					for (const comp of message.components) {
						updatedComponents[comp.id] = comp;
					}
					newSurfaces.set(message.surfaceId, {
						...existing,
						components: updatedComponents,
					});
				}
			}

			if (message.type === "deleteSurface") {
				newSurfaces.delete(message.surfaceId);
			}

			return newSurfaces;
		},
		[],
	);

	return { applyServerMessage };
}
