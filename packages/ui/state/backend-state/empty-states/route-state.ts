import type {
	CreateAppRoute,
	IAppRoute,
	IAppRouteState,
	UpdateAppRoute,
} from "../route-state";

export class EmptyRouteState implements IAppRouteState {
	getRoutes(_appId: string): Promise<IAppRoute[]> {
		throw new Error("Method not implemented.");
	}
	getRouteByPath(_appId: string, _path: string): Promise<IAppRoute | null> {
		throw new Error("Method not implemented.");
	}
	getDefaultRoute(_appId: string): Promise<IAppRoute | null> {
		throw new Error("Method not implemented.");
	}
	createRoute(_appId: string, _route: CreateAppRoute): Promise<IAppRoute> {
		throw new Error("Method not implemented.");
	}
	updateRoute(
		_appId: string,
		_routeId: string,
		_route: UpdateAppRoute,
	): Promise<IAppRoute> {
		throw new Error("Method not implemented.");
	}
	deleteRoute(_appId: string, _routeId: string): Promise<void> {
		throw new Error("Method not implemented.");
	}
}
