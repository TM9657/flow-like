import contract from "../../../core/a2ui-schema/src/excalidraw-event-schemas.json";

/** The portable .excalidraw document passed to load and save workflows. */
export interface ExcalidrawScene {
	type: "excalidraw";
	version: number;
	source: string;
	elements: Record<string, unknown>[];
	appState: Record<string, unknown>;
	files: Record<string, Record<string, unknown>>;
}

export type ExcalidrawEventName = keyof typeof contract.events;
type SceneDetails = { value: ExcalidrawScene; scene: ExcalidrawScene };
export type ExcalidrawEventDetails = {
	load: SceneDetails;
	save: SceneDetails & { changeCount: number };
	autoSave: SceneDetails & { changeCount: number };
	dirtyChange: { dirty: boolean };
	error: { message: string; operation: "load" | "save" | "autoSave" };
};

export function getExcalidrawEventContextSchema(
	eventName: string,
): Record<string, unknown> | undefined {
	if (!Object.hasOwn(contract.events, eventName)) return undefined;
	const definition = contract.events[eventName as ExcalidrawEventName];
	return {
		$schema: contract.$schema,
		$defs: contract.$defs,
		...contract.$defs[definition as keyof typeof contract.$defs],
	};
}
