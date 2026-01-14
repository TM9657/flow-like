import { createId } from "@paralleldrive/cuid2";
import { type AppRouteRecord, routeStorage } from "../../lib/idb-storage";
import type {
	CreateAppRoute,
	IAppRoute,
	IAppRouteState,
	UpdateAppRoute,
} from "./route-state";

/**
 * IndexedDB-backed implementation of IAppRouteState
 * Routes are persisted locally in the browser's IndexedDB
 */
export class IDBRouteState implements IAppRouteState {
	private toIAppRoute(record: AppRouteRecord): IAppRoute {
		return {
			id: record.id,
			appId: record.appId,
			path: record.path,
			targetType: record.targetType,
			pageId: record.pageId,
			boardId: record.boardId,
			pageVersion: record.pageVersion,
			eventId: record.eventId,
			priority: record.priority,
			label: record.label,
			icon: record.icon,
			createdAt: record.createdAt,
			updatedAt: record.updatedAt,
		};
	}

	async getRoutes(appId: string): Promise<IAppRoute[]> {
		const records = await routeStorage.getRoutes(appId);
		return records
			.map((r) => this.toIAppRoute(r))
			.sort((a, b) => a.priority - b.priority);
	}

	async getRouteByPath(appId: string, path: string): Promise<IAppRoute | null> {
		const record = await routeStorage.getRouteByPath(appId, path);
		return record ? this.toIAppRoute(record) : null;
	}

	async getDefaultRoute(appId: string): Promise<IAppRoute | null> {
		const record = await routeStorage.getDefaultRoute(appId);
		return record ? this.toIAppRoute(record) : null;
	}

	async createRoute(appId: string, route: CreateAppRoute): Promise<IAppRoute> {
		const now = new Date().toISOString();
		const newRecord: AppRouteRecord = {
			id: createId(),
			appId,
			path: route.path,
			targetType: route.targetType,
			pageId: route.pageId,
			boardId: route.boardId,
			pageVersion: route.pageVersion,
			eventId: route.eventId,
			priority: route.priority ?? 0,
			label: route.label,
			icon: route.icon,
			createdAt: now,
			updatedAt: now,
		};

		const created = await routeStorage.addRoute(appId, newRecord);
		return this.toIAppRoute(created);
	}

	async updateRoute(
		appId: string,
		routeId: string,
		route: UpdateAppRoute,
	): Promise<IAppRoute> {
		const updated = await routeStorage.updateRoute(appId, routeId, {
			path: route.path,
			targetType: route.targetType,
			pageId: route.pageId,
			boardId: route.boardId,
			pageVersion: route.pageVersion,
			eventId: route.eventId,
			priority: route.priority,
			label: route.label,
			icon: route.icon,
		});
		return this.toIAppRoute(updated);
	}

	async deleteRoute(appId: string, routeId: string): Promise<void> {
		await routeStorage.deleteRoute(appId, routeId);
	}
}
