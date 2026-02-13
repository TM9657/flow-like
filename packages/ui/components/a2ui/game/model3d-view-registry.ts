export interface Model3DViewState {
	cameraPosition: [number, number, number];
	cameraTarget: [number, number, number];
}

const viewRegistry = new Map<string, Model3DViewState>();

export function setModel3DView(
	componentId: string,
	view: Model3DViewState,
): void {
	viewRegistry.set(componentId, view);
}

export function getModel3DView(
	componentId: string,
): Model3DViewState | undefined {
	return viewRegistry.get(componentId);
}
