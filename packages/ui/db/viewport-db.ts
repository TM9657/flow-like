import Dexie, { type EntityTable } from "dexie";

export interface IViewportRecord {
	id: string; // `${appId}:${boardId}:${layerPath ?? 'root'}`
	appId: string;
	boardId: string;
	layerPath: string;
	x: number;
	y: number;
	zoom: number;
	updatedAt: number;
}

const viewportDb = new Dexie("Viewport-DB") as Dexie & {
	viewports: EntityTable<IViewportRecord, "id">;
};

viewportDb.version(1).stores({
	viewports: "&id, appId, boardId, layerPath, updatedAt",
});

export { viewportDb };

export function viewportKey(
	appId: string,
	boardId: string,
	layerPath?: string,
) {
	return `${appId}:${boardId}:${layerPath ?? "root"}`;
}
