import {
	GEOMETRY_KINDS,
	type Geometry,
	type GeometryKind,
	normalizeGeometryValue,
} from "./geometry";
import { isWgs84GeometryMetadata } from "./geometry-columns";

export interface GeometryDisplay {
	kind: GeometryKind | null;
	geometry: Geometry | null;
	knownCrs: boolean;
	error: string | null;
	positions: number;
	parts: number;
	holes: number;
	bounds: [number, number, number, number] | null;
	point: number[] | null;
	paths: { d: string; polygon: boolean }[];
	points: [number, number][];
}

const MAX_SKETCH_SHAPES = 32;
const MAX_SKETCH_VERTICES = 512;

/** A small coordinate sketch, bounded independently of the full geometry shown on the map. */
export function describeGeometry(
	value: unknown,
	metadata?: Record<string, unknown>,
): GeometryDisplay {
	return describe(value, isWgs84GeometryMetadata(metadata));
}

/** Flow pin and variable geometries are WGS 84 longitude/latitude by contract. */
export function describeFlowGeometry(value: unknown): GeometryDisplay {
	return describe(value, true);
}

function describe(value: unknown, knownCrs: boolean): GeometryDisplay {
	const kind =
		value &&
		typeof value === "object" &&
		"type" in value &&
		GEOMETRY_KINDS.includes(value.type as GeometryKind)
			? (value.type as GeometryKind)
			: null;
	const display: GeometryDisplay = {
		kind,
		geometry: null,
		knownCrs,
		error: null,
		positions: 0,
		parts: 0,
		holes: 0,
		bounds: null,
		point: null,
		paths: [],
		points: [],
	};
	// An extension tag alone does not establish coordinate order or units.
	if (!display.knownCrs) return display;
	try {
		display.geometry = normalizeGeometryValue(value) as Geometry;
	} catch (error) {
		display.error = error instanceof Error ? error.message : String(error);
		return display;
	}

	const shapes: { rings: number[][][]; polygon: boolean }[] = [];
	const dots: number[][] = [];
	let west = Number.POSITIVE_INFINITY;
	let south = Number.POSITIVE_INFINITY;
	let east = Number.NEGATIVE_INFINITY;
	let north = Number.NEGATIVE_INFINITY;
	const count = (positions: number[][]) => {
		for (const [longitude, latitude] of positions) {
			display.positions++;
			west = Math.min(west, longitude);
			east = Math.max(east, longitude);
			south = Math.min(south, latitude);
			north = Math.max(north, latitude);
		}
	};
	const line = (coordinates: number[][]) => {
		display.parts++;
		count(coordinates);
		if (shapes.length < MAX_SKETCH_SHAPES)
			shapes.push({ rings: [coordinates], polygon: false });
	};
	const polygon = (rings: number[][][]) => {
		display.parts++;
		display.holes += Math.max(0, rings.length - 1);
		for (const ring of rings) count(ring.slice(0, -1));
		if (shapes.length < MAX_SKETCH_SHAPES)
			shapes.push({ rings, polygon: true });
	};
	const visit = (geometry: Geometry): void => {
		switch (geometry.type) {
			case "Point":
				display.parts++;
				count([geometry.coordinates]);
				if (dots.length < MAX_SKETCH_SHAPES) dots.push(geometry.coordinates);
				break;
			case "MultiPoint":
				for (const coordinates of geometry.coordinates)
					visit({ type: "Point", coordinates });
				break;
			case "LineString":
				line(geometry.coordinates);
				break;
			case "MultiLineString":
				for (const coordinates of geometry.coordinates) line(coordinates);
				break;
			case "Polygon":
				polygon(geometry.coordinates);
				break;
			case "MultiPolygon":
				for (const coordinates of geometry.coordinates) polygon(coordinates);
				break;
			case "GeometryCollection":
				for (const geometryPart of geometry.geometries) visit(geometryPart);
				break;
		}
	};
	visit(display.geometry);
	if (!Number.isFinite(west)) return display;
	display.bounds = [west, south, east, north];
	if (display.geometry.type === "Point")
		display.point = display.geometry.coordinates;
	const scale = 48 / (Math.max(east - west, north - south) || 1);
	const project = ([x, y]: number[]): [number, number] => [
		32 + (x - (west + east) / 2) * scale,
		32 - (y - (south + north) / 2) * scale,
	];
	let remaining = MAX_SKETCH_VERTICES;
	for (const shape of shapes) {
		let d = "";
		for (const ring of shape.rings) {
			if (remaining < 4) break;
			const limit = Math.min(remaining, 96);
			const step = Math.max(1, Math.ceil((ring.length - 1) / (limit - 1)));
			const sampled = ring.filter((_, index) => index % step === 0);
			if (sampled.at(-1) !== ring.at(-1)) sampled.push(ring[ring.length - 1]);
			remaining -= sampled.length;
			d += sampled
				.map((position, index) => {
					const [x, y] = project(position);
					return `${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
				})
				.join(" ");
			if (shape.polygon) d += "Z ";
		}
		if (
			d &&
			!display.paths.some(
				(path) => path.d === d && path.polygon === shape.polygon,
			)
		)
			display.paths.push({ d, polygon: shape.polygon });
	}
	display.points = [
		...new Map(
			dots.map(project).map((point) => [point.join(","), point]),
		).values(),
	];
	return display;
}

const coordinateFormat = new Intl.NumberFormat("en", {
	maximumFractionDigits: 5,
});

export function formatGeometryCoordinate(value: number): string {
	return coordinateFormat.format(value);
}
