import { describe, expect, test } from "bun:test";
import {
	geometrySchemaField,
	normalizeSchemaGeometryValues,
} from "./geometry-schema";
import { IValueType } from "./schema/flow/pin";

const point = { type: "Point", coordinates: [13, 52] };
const annotation = {
	$id: "urn:flow-like:geometry:v1:Point",
	"x-flow-like-type": "geometry",
	"x-geometry": "Point",
	type: "object",
};
const schema = {
	type: "object",
	properties: { location: { $ref: "#/$defs/point" } },
	required: ["location"],
	$defs: { point: annotation },
};

describe("Geometry fields in interface schemas", () => {
	test("recognizes explicit annotations and resolved containers only", () => {
		expect(
			geometrySchemaField(schema.properties.location, schema)?.schema,
		).toContain('"x-geometry":"Point"');
		expect(
			geometrySchemaField({ type: "array", items: annotation })?.valueType,
		).toBe(IValueType.Array);
		expect(
			geometrySchemaField({
				type: "array",
				uniqueItems: true,
				items: annotation,
			})?.valueType,
		).toBe(IValueType.HashSet);
		expect(
			geometrySchemaField({ type: "object", additionalProperties: annotation })
				?.valueType,
		).toBe(IValueType.HashMap);
		expect(
			geometrySchemaField({
				type: "object",
				properties: { type: { const: "Point" } },
			}),
		).toBeUndefined();
	});
	test("validates required Geometry fields without inferring ordinary Struct values", () => {
		expect(
			normalizeSchemaGeometryValues(
				{ location: point, note: { type: "Point" } },
				schema,
				schema,
				false,
			),
		).toEqual({ location: point, note: { type: "Point" } });
		expect(() =>
			normalizeSchemaGeometryValues({}, schema, schema, false),
		).toThrow();
		expect(() =>
			normalizeSchemaGeometryValues(
				{ location: { type: "Polygon", coordinates: [] } },
				schema,
				schema,
				false,
			),
		).toThrow();
		expect(normalizeSchemaGeometryValues({}, schema)).toEqual({});
	});
});
