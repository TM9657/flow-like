import { useReactFlow } from "@xyflow/react";
import { useCallback, useEffect } from "react";
import { viewportDb, viewportKey } from "../db/viewport-db";

interface UseViewportManagerProps {
	appId: string;
	boardId: string;
	layerPath: string | undefined;
	nodesLength: number;
}

export function useViewportManager({
	appId,
	boardId,
	layerPath,
	nodesLength,
}: UseViewportManagerProps) {
	const { getViewport, setViewport, fitView } = useReactFlow();

	const saveViewport = useCallback(async () => {
		try {
			const vp = getViewport();
			await viewportDb.viewports.put({
				id: viewportKey(appId, boardId, layerPath),
				appId,
				boardId,
				layerPath: layerPath ?? "root",
				x: vp.x,
				y: vp.y,
				zoom: vp.zoom,
				updatedAt: Date.now(),
			});
		} catch {
			// no-op
		}
	}, [appId, boardId, layerPath, getViewport]);

	useEffect(() => {
		let active = true;

		const restore = async () => {
			const rec = await viewportDb.viewports.get(
				viewportKey(appId, boardId, layerPath),
			);
			if (!active) return;

			if (rec) {
				setViewport({ x: rec.x, y: rec.y, zoom: rec.zoom });
			} else {
				fitView({ duration: 300 });
			}
		};

		restore();

		return () => {
			active = false;
		};
	}, [appId, boardId, layerPath, setViewport, fitView, nodesLength]);

	return { saveViewport };
}
