import { describe, expect, test } from "bun:test";
import { geoArrowIndexKind } from "./geoarrow-index";

function coordinateType(axes = ["x", "y"]) {
	return { Struct: axes.map((name) => ({ name, data_type: "Float64" })) };
}

function geometry(extension: string, data_type: unknown) {
	return {
		data_type,
		metadata: { "ARROW:extension:name": extension },
	};
}

function nested(dataType: unknown, depth: number, listType = "List"): unknown {
	let current = dataType;
	for (let level = 0; level < depth; level++) {
		current = { [listType]: { name: "vertices", data_type: current } };
	}
	return current;
}

describe("GeoArrow spatial index eligibility", () => {
	test.each([
		["geoarrow.point", 0],
		["geoarrow.linestring", 1],
		["geoarrow.multipoint", 1],
		["geoarrow.polygon", 2],
		["geoarrow.multilinestring", 2],
		["geoarrow.multipolygon", 3],
	] as const)("accepts separated %s coordinates", (extension, depth) => {
		for (const listType of ["List", "LargeList"]) {
			expect(
				geoArrowIndexKind(
					geometry(extension, nested(coordinateType(), depth, listType)),
				),
			).toBe("geometry");
		}
	});

	test("accepts canonical coordinate dimensions and rectangle bounds", () => {
		for (const axes of [
			["x", "y"],
			["x", "y", "z"],
			["x", "y", "m"],
			["x", "y", "z", "m"],
		]) {
			expect(
				geoArrowIndexKind(geometry("geoarrow.point", coordinateType(axes))),
			).toBe("geometry");
			const bounds = [
				...axes.map((axis) => `${axis}min`),
				...axes.map((axis) => `${axis}max`),
			];
			expect(
				geoArrowIndexKind(geometry("geoarrow.box", coordinateType(bounds))),
			).toBe("geometry");
		}
	});

	test.each([
		["geoarrow.wkb", "Binary"],
		["geoarrow.wkb", "LargeBinary"],
		["geoarrow.wkb", "BinaryView"],
		["geoarrow.wkt", "Utf8"],
		["geoarrow.wkt", "LargeUtf8"],
		["geoarrow.wkt", "Utf8View"],
	] as const)("accepts %s with %s storage", (extension, dataType) => {
		expect(geoArrowIndexKind(geometry(extension, dataType))).toBe("geometry");
	});

	test("rejects interleaved points before and after Lance renames their child", () => {
		for (const name of ["xy", "item"]) {
			const interleaved = {
				FixedSizeList: [{ name, data_type: "Float64" }, 2],
			};
			expect(geoArrowIndexKind(geometry("geoarrow.point", interleaved))).toBe(
				"unsupported-geometry",
			);
			expect(
				geoArrowIndexKind(
					geometry("geoarrow.linestring", nested(interleaved, 1)),
				),
			).toBe("unsupported-geometry");
		}
	});

	test("rejects mixed list offset widths and mismatched geometry storage", () => {
		const mixed = nested(nested(coordinateType(), 1, "LargeList"), 1, "List");
		expect(geoArrowIndexKind(geometry("geoarrow.polygon", mixed))).toBe(
			"unsupported-geometry",
		);
		expect(geoArrowIndexKind(geometry("geoarrow.wkb", "Utf8"))).toBe(
			"unsupported-geometry",
		);
		expect(
			geoArrowIndexKind(geometry("geoarrow.point", coordinateType(["a", "b"]))),
		).toBe("unsupported-geometry");
		expect(
			geoArrowIndexKind(
				geometry("geoarrow.point", {
					Struct: [
						{ name: "x", data_type: "Float32" },
						{ name: "y", data_type: "Float32" },
					],
				}),
			),
		).toBe("unsupported-geometry");
	});

	test("requires recognized GeoArrow metadata", () => {
		expect(geoArrowIndexKind({ data_type: coordinateType() })).toBeUndefined();
		expect(
			geoArrowIndexKind(geometry("unrelated.point", coordinateType())),
		).toBeUndefined();
		expect(
			geoArrowIndexKind(geometry("geoarrow.unknown", coordinateType())),
		).toBe("unsupported-geometry");
	});
});
