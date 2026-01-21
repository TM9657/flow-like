import { invoke } from "@tauri-apps/api/core";
import type { IAppRouteState, IRouteMapping } from "@tm9657/flow-like-ui";
import type { TauriBackend } from "../tauri-provider";

export class RouteState implements IAppRouteState {
	constructor(private readonly backend: TauriBackend) {}

	async getRoutes(appId: string): Promise<IRouteMapping[]> {
		return invoke<IRouteMapping[]>("get_app_routes", { appId });
	}

	async getRouteByPath(
		appId: string,
		path: string,
	): Promise<IRouteMapping | null> {
		return invoke<IRouteMapping | null>("get_app_route_by_path", {
			appId,
			path,
		});
	}

	async getDefaultRoute(appId: string): Promise<IRouteMapping | null> {
		return invoke<IRouteMapping | null>("get_default_app_route", { appId });
	}

	async setRoute(
		appId: string,
		path: string,
		eventId: string,
	): Promise<IRouteMapping> {
		return invoke<IRouteMapping>("set_app_route", { appId, path, eventId });
	}

	async setRoutes(
		appId: string,
		routes: Record<string, string>,
	): Promise<IRouteMapping[]> {
		return invoke<IRouteMapping[]>("set_app_routes", { appId, routes });
	}

	async deleteRouteByPath(appId: string, path: string): Promise<void> {
		return invoke("delete_app_route_by_path", { appId, path });
	}

	async deleteRouteByEvent(appId: string, eventId: string): Promise<void> {
		return invoke("delete_app_route_by_event", { appId, eventId });
	}
}
