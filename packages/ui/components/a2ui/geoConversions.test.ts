import { describe, expect, test } from "bun:test";
import {
	applyElementUpdate,
	normalizeGeoMapViewport,
} from "./apply-a2ui-message";
import {
	geoMapViewportValue,
	mapEventCoordinate,
	normalizeGeoMapMarkers,
	normalizeGeoMapRoutes,
} from "./geoConversions";
import type { SurfaceComponent } from "./types";

const point = { type: "Point", coordinates: [13.405, 52.52] };
const line = {
	type: "LineString",
	coordinates: [
		[13.405, 52.52],
		[2.3522, 48.8566],
	],
};

describe("native geometry map adapters", () => {
	test("Point coordinates and outgoing events keep the established axis order", () => {
		const marker = normalizeGeoMapMarkers(point)[0];
		expect(marker.coordinate).toEqual({ latitude: 52.52, longitude: 13.405 });
		expect(mapEventCoordinate(13.405, 52.52)).toEqual(marker.coordinate);
		expect(normalizeGeoMapRoutes(point)).toEqual([]);
	});

	test("line and polygon outlines retain all positions and holes", () => {
		expect(normalizeGeoMapRoutes(line)[0].coordinates[1]).toEqual({
			latitude: 48.8566,
			longitude: 2.3522,
		});
		const polygon = {
			type: "Polygon",
			coordinates: [
				[
					[0, 0],
					[4, 0],
					[4, 4],
					[0, 4],
					[0, 0],
				],
				[
					[1, 1],
					[1, 3],
					[3, 3],
					[3, 1],
					[1, 1],
				],
			],
		};
		const routes = normalizeGeoMapRoutes(polygon);
		expect(routes).toHaveLength(2);
		expect(routes[0].coordinates).toHaveLength(5);
		expect(routes[1].coordinates[0]).toEqual(routes[1].coordinates[4]);
	});

	test("collections split native points and lines without changing source values", () => {
		const geometry = { type: "GeometryCollection", geometries: [point, line] };
		const snapshot = JSON.stringify(geometry);
		expect(normalizeGeoMapMarkers(geometry)).toHaveLength(1);
		expect(normalizeGeoMapRoutes(geometry)).toHaveLength(1);
		expect(JSON.stringify(geometry)).toBe(snapshot);
	});

	test("legacy styled markers and rich route wrappers keep their metadata", () => {
		const marker = {
			id: "berlin",
			coordinate: { latitude: 52.52, longitude: 13.405 },
			color: "orange",
			label: "Berlin",
			draggable: true,
		};
		expect(normalizeGeoMapMarkers([marker])[0]).toEqual(marker);
		const location = {
			...marker,
			type: "city",
			display_name: "Berlin, Germany",
		};
		expect(normalizeGeoMapMarkers(location)[0]).toMatchObject(location);
		const route = {
			id: "trip",
			distance: 10,
			geometry: {
				points: [marker.coordinate, { latitude: 48.8566, longitude: 2.3522 }],
			},
			legs: [{ summary: "retain" }],
			color: "orange",
		};
		const result = normalizeGeoMapRoutes(route)[0];
		expect(result.coordinates).toEqual(route.geometry.points);
		expect(result).toMatchObject({
			color: "orange",
			distance: 10,
			legs: route.legs,
		});
		expect(route.geometry.points).toHaveLength(2);
	});

	test("flat, nested, wrapped and Point viewports agree", () => {
		for (const raw of [
			{ latitude: 52.52, longitude: 13.405 },
			{ center: { latitude: 52.52, longitude: 13.405 } },
			point,
			{ literalJson: JSON.stringify(point) },
		])
			expect(geoMapViewportValue(raw)?.center).toEqual({
				latitude: 52.52,
				longitude: 13.405,
			});
		const bound = normalizeGeoMapViewport(point) as { literalJson: string };
		expect(JSON.parse(bound.literalJson).center).toEqual({
			latitude: 52.52,
			longitude: 13.405,
		});
	});

	test("invalid geometry stays an explicit validation error", () => {
		expect(() =>
			normalizeGeoMapMarkers({ type: "Point", coordinates: [13, 52, 9] }),
		).toThrow();
		expect(() =>
			normalizeGeoMapMarkers({
				type: "Point",
				coordinate: { longitude: 13, latitude: 52 },
			}),
		).toThrow();
		expect(() =>
			normalizeGeoMapRoutes({ type: "LineString", coordinates: [[0, 0]] }),
		).toThrow();
		expect(() =>
			geoMapViewportValue({ latitude: Number.NaN, longitude: 0 }),
		).toThrow();
		expect(() =>
			geoMapViewportValue({ type: "Point", coordinates: [0, 91] }),
		).toThrow();
		expect(
			geoMapViewportValue({ longitude: 181, latitude: 52 })?.center.longitude,
		).toBe(181);
	});

	test("native Geometry update binds both markers and outlines", () => {
		const component = {
			id: "map",
			component: { type: "geoMap" },
		} as SurfaceComponent;
		const result = applyElementUpdate(component, {
			type: "setGeoMapGeometry",
			geometry: point,
		});
		expect(result.component).toMatchObject({
			markers: { literalJson: JSON.stringify(point) },
			routes: { literalJson: JSON.stringify(point) },
		});
	});
});
