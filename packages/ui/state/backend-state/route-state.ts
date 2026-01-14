import type { Version } from "./widget-state";

export type RouteTargetType = "page" | "event";

export interface IAppRoute {
	id: string;
	appId: string;
	path: string;
	targetType: RouteTargetType;
	pageId?: string;
	boardId?: string;
	pageVersion?: Version;
	eventId?: string;
	priority: number;
	label?: string;
	icon?: string;
	createdAt: string;
	updatedAt: string;
}

export interface CreateAppRoute {
	path: string;
	targetType: RouteTargetType;
	pageId?: string;
	boardId?: string;
	pageVersion?: Version;
	eventId?: string;
	priority?: number;
	label?: string;
	icon?: string;
}

export interface UpdateAppRoute {
	path?: string;
	targetType?: RouteTargetType;
	pageId?: string;
	boardId?: string;
	pageVersion?: Version;
	eventId?: string;
	priority?: number;
	label?: string;
	icon?: string;
}

export interface IAppRouteState {
	getRoutes(appId: string): Promise<IAppRoute[]>;
	getRouteByPath(appId: string, path: string): Promise<IAppRoute | null>;
	getDefaultRoute(appId: string): Promise<IAppRoute | null>;
	createRoute(appId: string, route: CreateAppRoute): Promise<IAppRoute>;
	updateRoute(
		appId: string,
		routeId: string,
		route: UpdateAppRoute,
	): Promise<IAppRoute>;
	deleteRoute(appId: string, routeId: string): Promise<void>;
}
