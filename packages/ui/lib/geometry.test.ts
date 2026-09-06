import { describe, expect, test } from "bun:test";
import {
	GEOMETRY_KINDS,
	MAX_GEOMETRY_DEPTH,
	MAX_GEOMETRY_JSON_BYTES,
	geometryKindFromSchema,
	geometryMarker,
	geometrySchemasCompatible,
	normalizeGeometryValue,
	parseGeometryText,
} from "./geometry";
import { IValueType } from "./schema/flow/pin";

const point = { type: "Point", coordinates: [13.405, 52.52] };
const ring = [
	[0, 0],
	[1, 0],
	[1, 1],
	[0, 0],
];
const geometries = [
	point,
	{
		type: "LineString",
		coordinates: [
			[0, 0],
			[1, 1],
		],
	},
	{ type: "Polygon", coordinates: [ring] },
	{ type: "MultiPoint", coordinates: [[0, 0]] },
	{
		type: "MultiLineString",
		coordinates: [
			[
				[0, 0],
				[1, 1],
			],
		],
	},
	{ type: "MultiPolygon", coordinates: [[ring]] },
	{ type: "GeometryCollection", geometries: [point] },
];

describe("geometry wire contract", () => {
	test("all subtypes and containers validate without mistaking a multi-geometry for an array", () => {
		for (const [index, kind] of GEOMETRY_KINDS.entries()) {
			const value = geometries[index];
			const schema = geometryMarker(kind);
			expect(normalizeGeometryValue(value, { schema })).toEqual(value);
			expect(
				normalizeGeometryValue([value], {
					schema,
					valueType: IValueType.Array,
				}),
			).toEqual([value]);
			expect(
				normalizeGeometryValue(
					{ location: value },
					{ schema, valueType: IValueType.HashMap },
				),
			).toEqual({ location: value });
		}
		expect(() =>
			normalizeGeometryValue(geometries[3], { valueType: IValueType.Array }),
		).toThrow();
	});

	test("null is unset only when requested; geometry container members cannot be null", () => {
		expect(normalizeGeometryValue(null, { allowUnset: true })).toBeNull();
		expect(() => normalizeGeometryValue(null)).toThrow("required");
		expect(() =>
			normalizeGeometryValue([null], {
				valueType: IValueType.Array,
				allowUnset: true,
			}),
		).toThrow();
		expect(parseGeometryText("", { allowUnset: true })).toBeNull();
	});

	test("empty multi-geometries and collections are valid; empty scalar forms and components fail", () => {
		for (const type of ["MultiPoint", "MultiLineString", "MultiPolygon"])
			expect(normalizeGeometryValue({ type, coordinates: [] })).toEqual({
				type,
				coordinates: [],
			});
		expect(
			normalizeGeometryValue({ type: "GeometryCollection", geometries: [] }),
		).toEqual({ type: "GeometryCollection", geometries: [] });
		for (const type of ["Point", "LineString", "Polygon"])
			expect(() => normalizeGeometryValue({ type, coordinates: [] })).toThrow();
		for (const type of ["MultiLineString", "MultiPolygon"])
			expect(() =>
				normalizeGeometryValue({ type, coordinates: [[]] }),
			).toThrow();
	});

	test("rejects non-finite, projected, 3D, malformed, unclosed, and wrapped values", () => {
		for (const coordinates of [
			[NaN, 0],
			[Infinity, 0],
			[0, -Infinity],
			[181, 0],
			[0, 91],
			[1, 2, 3],
			["1", 2],
		])
			expect(() =>
				normalizeGeometryValue({ type: "Point", coordinates }),
			).toThrow();
		expect(() => normalizeGeometryValue({ ...point, crs: null })).toThrow(
			"CRS",
		);
		expect(() =>
			normalizeGeometryValue({ ...point, geometries: [] }),
		).toThrow();
		expect(() =>
			normalizeGeometryValue({
				type: "GeometryCollection",
				geometries: [],
				coordinates: [],
			}),
		).toThrow();
		expect(() =>
			normalizeGeometryValue({
				type: "Feature",
				geometry: point,
				properties: {},
			}),
		).toThrow("extract");
		expect(() =>
			normalizeGeometryValue({
				type: "Polygon",
				coordinates: [
					[
						[0, 0],
						[1, 0],
						[1, 1],
						[0, 1],
					],
				],
			}),
		).toThrow("closed");
		expect(() =>
			normalizeGeometryValue(point, { schema: geometryMarker("Polygon") }),
		).toThrow("Polygon");
	});

	test("normalizes ring winding, preserves foreign members and validates bbox", () => {
		const input = {
			type: "Polygon",
			coordinates: [[...ring].reverse()],
			note: { source: "survey" },
			bbox: [0, 0, 1, 1],
		};
		expect(normalizeGeometryValue(input)).toEqual({
			...input,
			coordinates: [ring],
		});
		expect(input.coordinates[0]).toEqual([...ring].reverse());
		expect(
			normalizeGeometryValue({ ...point, bbox: [170, -20, -170, 20] }),
		).toBeTruthy();
		expect(() =>
			normalizeGeometryValue({ ...point, bbox: [0, 20, 1, -20] }),
		).toThrow();
		expect(() => normalizeGeometryValue({ ...point, foreign: NaN })).toThrow(
			"finite",
		);
	});

	test("enforces depth and byte limits before accepting untrusted values", () => {
		let nested: unknown = "leaf";
		for (let i = 0; i < MAX_GEOMETRY_DEPTH; i++) nested = { child: nested };
		expect(() => normalizeGeometryValue({ ...point, nested })).toThrow(
			"nesting",
		);
		expect(() =>
			parseGeometryText(" ".repeat(MAX_GEOMETRY_JSON_BYTES + 1)),
		).toThrow("size");
		expect(() =>
			normalizeGeometryValue({
				...point,
				note: "x".repeat(MAX_GEOMETRY_JSON_BYTES),
			}),
		).toThrow("size");
	});

	test("set equality compares JSON values, ignoring object key order", () => {
		expect(
			normalizeGeometryValue(
				[point, { coordinates: point.coordinates, type: "Point" }],
				{ valueType: IValueType.HashSet },
			),
		).toEqual([point]);
	});
});

describe("geometry subtype markers", () => {
	test("frozen marker spelling and interned refs round-trip", () => {
		expect(geometryMarker("Point")).toBe(
			'{"$id":"flow:geometry","x-geometry":"Point"}',
		);
		expect(geometryMarker()).toBeNull();
		expect(
			geometryKindFromSchema("hash", { hash: geometryMarker("Point")! }),
		).toBe("Point");
	});

	test("rejects unknown, generic, ordinary, extended, and cyclic markers", () => {
		for (const marker of [
			'{"$id":"flow:geometry","x-geometry":"Any"}',
			'{"type":"object"}',
			'{"$id":"flow:geometry","x-geometry":"Point","title":"P"}',
			"broken",
		])
			expect(() => geometryKindFromSchema(marker)).toThrow();
		expect(() => geometryKindFromSchema("a", { a: "b", b: "a" })).toThrow(
			"Circular",
		);
	});

	test("complete directional subtype matrix", () => {
		for (const source of [null, ...GEOMETRY_KINDS])
			for (const target of [null, ...GEOMETRY_KINDS])
				expect(
					geometrySchemasCompatible(
						geometryMarker(source),
						geometryMarker(target),
					),
				).toBe(target === null || source === target);
	});
});

test("an explicit empty schema is not the absent any-geometry marker", () => {
	expect(() => geometryKindFromSchema("")).toThrow(
		"Invalid geometry subtype marker",
	);
});
