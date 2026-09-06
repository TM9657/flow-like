import { describe, expect, test } from "bun:test";
import { describeGeometry, formatGeometryCoordinate } from "./geometry-display";

const metadata = {
	"ARROW:extension:name": "geoarrow.wkb",
	"ARROW:extension:metadata": JSON.stringify({
		crs: "EPSG:4326",
		edges: "planar",
	}),
};
const point = { type: "Point", coordinates: [13.405, 52.52] };

describe("Geometry value presentation", () => {
	test("keeps longitude first and places a point in the center of its sketch", () => {
		const display = describeGeometry(point, metadata);
		expect(display.point).toEqual([13.405, 52.52]);
		expect(display.bounds).toEqual([13.405, 52.52, 13.405, 52.52]);
		expect(display.points).toEqual([[32, 32]]);
		expect(formatGeometryCoordinate(13.40500001)).toBe("13.405");
	});

	test("draws polygon holes and counts vertices without closing duplicates", () => {
		const display = describeGeometry(
			{
				type: "Polygon",
				coordinates: [
					[
						[0, 0],
						[8, 0],
						[8, 8],
						[0, 8],
						[0, 0],
					],
					[
						[2, 2],
						[2, 4],
						[4, 4],
						[4, 2],
						[2, 2],
					],
				],
			},
			metadata,
		);
		expect(display.error).toBeNull();
		expect(display.positions).toBe(8);
		expect(display.holes).toBe(1);
		expect(display.paths).toHaveLength(1);
		expect(display.paths[0].polygon).toBe(true);
		expect(display.paths[0].d.match(/Z/g)).toHaveLength(2);
	});

	test("includes nested collections and each multi geometry in its summary", () => {
		const display = describeGeometry(
			{
				type: "GeometryCollection",
				geometries: [
					point,
					{
						type: "GeometryCollection",
						geometries: [
							{
								type: "MultiLineString",
								coordinates: [
									[
										[0, 0],
										[1, 1],
									],
									[
										[3, 4],
										[5, 6],
									],
								],
							},
							{
								type: "MultiPolygon",
								coordinates: [
									[
										[
											[0, 0],
											[1, 0],
											[1, 1],
											[0, 0],
										],
									],
								],
							},
						],
					},
				],
			},
			metadata,
		);
		expect(display.parts).toBe(4);
		expect(display.positions).toBe(8);
		expect(display.paths).toHaveLength(3);
		expect(display.points).toHaveLength(1);
	});

	test("does not assume geographic coordinates from an extension tag or value shape", () => {
		for (const unsupported of [
			undefined,
			{ "ARROW:extension:name": "geoarrow.wkb" },
			{ ...metadata, "ARROW:extension:metadata": '{"crs":"EPSG:3857"}' },
			{
				...metadata,
				"ARROW:extension:metadata": '{"crs":"EPSG:4326","edges":"spherical"}',
			},
		]) {
			const display = describeGeometry(point, unsupported);
			expect(display.knownCrs).toBe(false);
			expect(display.geometry).toBeNull();
			expect(display.paths).toEqual([]);
			expect(display.points).toEqual([]);
		}
	});

	test("invalid declared values keep an error instead of a misleading map", () => {
		const display = describeGeometry(
			{ type: "Point", coordinates: [200, 500] },
			metadata,
		);
		expect(display.error).toContain("WGS 84");
		expect(display.geometry).toBeNull();
		expect(display.bounds).toBeNull();
	});

	test("large geometries keep exact counts while their inline sketches stay bounded", () => {
		const coordinates = Array.from({ length: 2000 }, (_, index) => [
			index / 20 - 50,
			Math.sin(index / 20) * 10,
		]);
		const display = describeGeometry(
			{
				type: "MultiLineString",
				coordinates: Array.from({ length: 12 }, () => coordinates),
			},
			metadata,
		);
		expect(display.error).toBeNull();
		expect(display.positions).toBe(24_000);
		expect(
			display.paths.flatMap(({ d }) => d.match(/[ML]/g) ?? []).length,
		).toBeLessThanOrEqual(512);
	});

	test("coincident points share a sketch mark without losing the point count", () => {
		const display = describeGeometry(
			{
				type: "MultiPoint",
				coordinates: [
					[1, 2],
					[1, 2],
				],
			},
			metadata,
		);
		expect(display.positions).toBe(2);
		expect(display.points).toHaveLength(1);
	});
});
