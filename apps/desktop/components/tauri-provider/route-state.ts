import { invoke } from "@tauri-apps/api/core";
import type {
	CreateAppRoute,
	IAppRoute,
	IAppRouteState,
	UpdateAppRoute,
} from "@tm9657/flow-like-ui";
import type { TauriBackend } from "../tauri-provider";

export class RouteState implements IAppRouteState {
	constructor(private readonly backend: TauriBackend) {}

	async getRoutes(appId: string): Promise<IAppRoute[]> {
		return invoke<IAppRoute[]>("get_app_routes", { appId });
	}

	async getRouteByPath(appId: string, path: string): Promise<IAppRoute | null> {
		return invoke<IAppRoute | null>("get_app_route_by_path", { appId, path });
	}

	async getDefaultRoute(appId: string): Promise<IAppRoute | null> {
		return invoke<IAppRoute | null>("get_default_app_route", { appId });
	}

	async createRoute(appId: string, route: CreateAppRoute): Promise<IAppRoute> {
		return invoke<IAppRoute>("create_app_route", { appId, route });
	}

	async updateRoute(
		appId: string,
		routeId: string,
		route: UpdateAppRoute,
	): Promise<IAppRoute> {
		return invoke<IAppRoute>("update_app_route", { appId, routeId, route });
	}

	async deleteRoute(appId: string, routeId: string): Promise<void> {
		return invoke("delete_app_route", { appId, routeId });
	}
}
