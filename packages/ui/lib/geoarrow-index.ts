export type GeoArrowIndexKind = "geometry" | "unsupported-geometry";

export const UNSUPPORTED_GEOARROW_INDEX_HELP =
	"R-Tree supports GeoArrow WKB/WKT and geometries with separate Float64 coordinate fields. Interleaved coordinates in fixed-size lists are not supported by the current Lance storage format.";

function record(value: unknown): Record<string, unknown> | undefined {
	return value !== null && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: undefined;
}

function hasCoordinateFields(dataType: unknown, rectangle = false): boolean {
	const fields = record(dataType)?.Struct;
	if (!Array.isArray(fields)) return false;
	const dimensions = [
		["x", "y"],
		["x", "y", "z"],
		["x", "y", "m"],
		["x", "y", "z", "m"],
	];
	return dimensions.some((axes) => {
		const names = rectangle
			? [
					...axes.map((axis) => `${axis}min`),
					...axes.map((axis) => `${axis}max`),
				]
			: axes;
		return (
			fields.length === names.length &&
			fields.every((field, index) => {
				const child = record(field);
				return child?.name === names[index] && child.data_type === "Float64";
			})
		);
	});
}

function hasSeparatedCoordinates(dataType: unknown, depth: number): boolean {
	if (depth === 0) return hasCoordinateFields(dataType);
	return ["List", "LargeList"].some((listType) => {
		let current = dataType;
		for (let level = 0; level < depth; level++) {
			const child = record(current)?.[listType];
			current = record(Array.isArray(child) ? child[0] : child)?.data_type;
		}
		return hasCoordinateFields(current);
	});
}

/** Restrict spatial indexes to GeoArrow layouts Lance can preserve on disk. */
export function geoArrowIndexKind(
	field: unknown,
): GeoArrowIndexKind | undefined {
	const arrowField = record(field);
	const extension = record(arrowField?.metadata)?.["ARROW:extension:name"];
	if (typeof extension !== "string" || !extension.startsWith("geoarrow.")) {
		return undefined;
	}
	const dataType = arrowField?.data_type;
	let supported = false;
	switch (extension) {
		case "geoarrow.point":
			supported = hasSeparatedCoordinates(dataType, 0);
			break;
		case "geoarrow.linestring":
		case "geoarrow.multipoint":
			supported = hasSeparatedCoordinates(dataType, 1);
			break;
		case "geoarrow.polygon":
		case "geoarrow.multilinestring":
			supported = hasSeparatedCoordinates(dataType, 2);
			break;
		case "geoarrow.multipolygon":
			supported = hasSeparatedCoordinates(dataType, 3);
			break;
		case "geoarrow.box":
			supported = hasCoordinateFields(dataType, true);
			break;
		case "geoarrow.wkb":
			supported = ["Binary", "LargeBinary", "BinaryView"].includes(
				String(dataType),
			);
			break;
		case "geoarrow.wkt":
			supported = ["Utf8", "LargeUtf8", "Utf8View"].includes(String(dataType));
			break;
	}
	return supported ? "geometry" : "unsupported-geometry";
}
