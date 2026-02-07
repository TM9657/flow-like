import type {
	GeoMapMarkerDef,
	GeoMapRouteDef,
	GeoMapViewport,
	GeoRouteResult,
	GeoSearchResult,
	GeoTripWaypoint,
} from "./types";

let idCounter = 0;
function nextId(prefix: string): string {
	return `${prefix}-${++idCounter}`;
}

export function routeResultToRouteDef(
	route: GeoRouteResult,
	options?: { id?: string; color?: string; width?: number; opacity?: number },
): GeoMapRouteDef {
	return {
		id: options?.id ?? nextId("route"),
		coordinates: route.geometry.points,
		color: options?.color,
		width: options?.width,
		opacity: options?.opacity,
	};
}

export function routeResultsToRouteDefs(
	routes: GeoRouteResult[],
	options?: { color?: string; width?: number; opacity?: number },
): GeoMapRouteDef[] {
	return routes.map((r, i) =>
		routeResultToRouteDef(r, { ...options, id: `route-${i}` }),
	);
}

export function searchResultToMarkerDef(
	result: GeoSearchResult,
	options?: { id?: string; color?: string; draggable?: boolean },
): GeoMapMarkerDef {
	return {
		id: options?.id ?? nextId("search"),
		coordinate: result.coordinate,
		label: result.display_name,
		popup: `${result.display_name} (${result.place_type})`,
		color: options?.color ?? "blue",
		draggable: options?.draggable ?? false,
	};
}

export function searchResultsToMarkerDefs(
	results: GeoSearchResult[],
	options?: { color?: string },
): GeoMapMarkerDef[] {
	return results.map((r, i) =>
		searchResultToMarkerDef(r, { ...options, id: `search-${i}` }),
	);
}

export function tripWaypointToMarkerDef(
	waypoint: GeoTripWaypoint,
	options?: { id?: string; color?: string; draggable?: boolean },
): GeoMapMarkerDef {
	return {
		id: options?.id ?? nextId("waypoint"),
		coordinate: waypoint.coordinate,
		label: waypoint.name || undefined,
		color: options?.color ?? "green",
		draggable: options?.draggable ?? false,
	};
}

export function tripWaypointsToMarkerDefs(
	waypoints: GeoTripWaypoint[],
	options?: { color?: string },
): GeoMapMarkerDef[] {
	return waypoints.map((w, i) =>
		tripWaypointToMarkerDef(w, { ...options, id: `waypoint-${i}` }),
	);
}

export function routeStepsToMarkerDefs(
	route: GeoRouteResult,
	options?: { color?: string },
): GeoMapMarkerDef[] {
	return route.legs.flatMap((leg, li) =>
		leg.steps.map((step, si) => ({
			id: `step-${li}-${si}`,
			coordinate: step.coordinate,
			label: step.name || undefined,
			popup: step.instruction,
			color: options?.color ?? "orange",
		})),
	);
}

export function searchResultToViewport(result: GeoSearchResult): GeoMapViewport {
	return {
		center: result.coordinate,
		zoom: result.bounding_box ? 14 : 12,
	};
}
