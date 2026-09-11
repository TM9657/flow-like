import { GEOMETRY_KINDS, type Geometry, type GeometryKind } from "./geometry";

export type GeometryPosition = [number, number];
export type GeometryRing = GeometryPosition[];
export type GeometryShape = GeometryRing[];
export type GeometryDraftKind = Exclude<GeometryKind, "GeometryCollection">;

/**
 * Editable form of one GeoJSON geometry. Rings stay open while editing and
 * polygons are closed on emit, so vertex lists never show the closing repeat.
 * Invariant: at least one shape, every shape has at least one ring.
 */
export interface GeometryDraft {
	kind: GeometryDraftKind;
	shapes: GeometryShape[];
}

export interface GeometryRingRef {
	shape: number;
	ring: number;
}

export interface GeometryDraftCapabilities {
	/** MultiLineString / MultiPolygon: parts can be added and removed. */
	multiShape: boolean;
	/** Polygon kinds: rings after the exterior are holes. */
	holes: boolean;
	/** Rings are closed on emit and rendered filled. */
	polygon: boolean;
	/** Point: exactly one position, placing replaces it. */
	singlePosition: boolean;
	/** Positions a ring needs before the geometry is valid. */
	minPositions: number;
}

export const GEOMETRY_DRAFT_KINDS = GEOMETRY_KINDS.filter(
	(kind): kind is GeometryDraftKind => kind !== "GeometryCollection",
);

const COORDINATE_PRECISION = 1e6;

export const FIRST_RING: GeometryRingRef = { shape: 0, ring: 0 };

export function roundCoordinate(value: number): number {
	return Math.round(value * COORDINATE_PRECISION) / COORDINATE_PRECISION;
}

/** Wraps longitude into [-180, 180], clamps latitude to [-90, 90] and rounds to ~0.1 m. */
export function clampPosition(position: readonly number[]): GeometryPosition {
	let longitude = position[0];
	if (longitude > 180 || longitude < -180)
		longitude = ((((longitude + 180) % 360) + 360) % 360) - 180;
	const latitude = Math.max(-90, Math.min(90, position[1]));
	return [roundCoordinate(longitude), roundCoordinate(latitude)];
}

export function draftCapabilities(
	kind: GeometryDraftKind,
): GeometryDraftCapabilities {
	const base = {
		multiShape: false,
		holes: false,
		polygon: false,
		singlePosition: false,
	};
	switch (kind) {
		case "Point":
			return { ...base, singlePosition: true, minPositions: 1 };
		case "MultiPoint":
			return { ...base, minPositions: 1 };
		case "LineString":
			return { ...base, minPositions: 2 };
		case "MultiLineString":
			return { ...base, multiShape: true, minPositions: 2 };
		case "Polygon":
			return { ...base, holes: true, polygon: true, minPositions: 3 };
		case "MultiPolygon":
			return {
				...base,
				multiShape: true,
				holes: true,
				polygon: true,
				minPositions: 3,
			};
	}
}

export function emptyGeometryDraft(
	kind: GeometryDraftKind = "Point",
): GeometryDraft {
	return { kind, shapes: [[[]]] };
}

const isPosition = (value: unknown): value is GeometryPosition =>
	Array.isArray(value) &&
	value.length === 2 &&
	value.every((n) => typeof n === "number" && Number.isFinite(n));

function mapAll<T>(
	items: unknown,
	convert: (item: unknown) => T | null,
): T[] | null {
	if (!Array.isArray(items)) return null;
	const result: T[] = [];
	for (const item of items) {
		const converted = convert(item);
		if (converted === null) return null;
		result.push(converted);
	}
	return result;
}

const toRing = (value: unknown): GeometryRing | null =>
	mapAll(value, (p) => (isPosition(p) ? [p[0], p[1]] : null));

const toRings = (value: unknown): GeometryShape | null =>
	mapAll(value, toRing);

function openRing(ring: GeometryRing): GeometryRing {
	const first = ring[0];
	const last = ring[ring.length - 1];
	return ring.length > 1 && first[0] === last[0] && first[1] === last[1]
		? ring.slice(0, -1)
		: ring;
}

function ensureShapes(draft: GeometryDraft): GeometryDraft {
	const shapes = draft.shapes.map((shape) => (shape.length ? shape : [[]]));
	return { kind: draft.kind, shapes: shapes.length ? shapes : [[[]]] };
}

/** `null` when the value is not a single editable geometry (GeometryCollection, malformed). */
export function geometryToDraft(geometry: unknown): GeometryDraft | null {
	if (typeof geometry !== "object" || geometry === null) return null;
	const { type, coordinates } = geometry as {
		type?: unknown;
		coordinates?: unknown;
	};
	const kind = type as GeometryDraftKind;
	if (!GEOMETRY_DRAFT_KINDS.includes(kind)) return null;
	let shapes: GeometryShape[] | null = null;
	switch (kind) {
		case "Point":
			shapes = isPosition(coordinates)
				? [[[[coordinates[0], coordinates[1]]]]]
				: null;
			break;
		case "MultiPoint":
		case "LineString": {
			const ring = toRing(coordinates);
			shapes = ring ? [[ring]] : null;
			break;
		}
		case "MultiLineString":
			shapes = mapAll(coordinates, toRing)?.map((line) => [line]) ?? null;
			break;
		case "Polygon": {
			const rings = toRings(coordinates);
			shapes = rings ? [rings.map(openRing)] : null;
			break;
		}
		case "MultiPolygon":
			shapes =
				mapAll(coordinates, toRings)?.map((rings) => rings.map(openRing)) ??
				null;
			break;
	}
	return shapes ? ensureShapes({ kind, shapes }) : null;
}

function closeRing(ring: GeometryRing): GeometryRing {
	return ring.length ? [...ring, ring[0]] : ring;
}

/** Empty holes and empty extra parts are editor scaffolding and are dropped. */
function polygonRings(shape: GeometryShape): GeometryRing[] {
	return shape
		.filter((ring, index) => index === 0 || ring.length > 0)
		.map(closeRing);
}

/** `null` when nothing has been placed yet. Validity is left to `normalizeGeometryValue`. */
export function draftToGeometry(draft: GeometryDraft): Geometry | null {
	const shapes = draft.shapes.filter((shape) =>
		shape.some((ring) => ring.length > 0),
	);
	if (shapes.length === 0) return null;
	switch (draft.kind) {
		case "Point":
			return { type: "Point", coordinates: shapes[0][0][0] };
		case "MultiPoint":
			return { type: "MultiPoint", coordinates: shapes[0][0] };
		case "LineString":
			return { type: "LineString", coordinates: shapes[0][0] };
		case "MultiLineString":
			return {
				type: "MultiLineString",
				coordinates: shapes.map((shape) => shape[0]),
			};
		case "Polygon":
			return { type: "Polygon", coordinates: polygonRings(shapes[0]) };
		case "MultiPolygon":
			return { type: "MultiPolygon", coordinates: shapes.map(polygonRings) };
	}
}

/** Re-shapes the placed positions for another kind, keeping as much structure as possible. */
export function convertGeometryDraft(
	draft: GeometryDraft,
	kind: GeometryDraftKind,
): GeometryDraft {
	if (draft.kind === kind) return draft;
	const rings = draft.shapes.flat().filter((ring) => ring.length > 0);
	const positions = rings.flat();
	const from = draftCapabilities(draft.kind);
	const to = draftCapabilities(kind);
	let shapes: GeometryShape[];
	if (to.singlePosition) shapes = [[positions.slice(0, 1)]];
	else if (!to.multiShape && !to.holes) shapes = [[positions]];
	else if (to.multiShape && !to.holes) shapes = rings.map((ring) => [ring]);
	else if (!to.multiShape) shapes = [rings];
	else
		shapes = from.holes
			? draft.shapes.filter((shape) => shape.some((ring) => ring.length > 0))
			: rings.map((ring) => [ring]);
	return ensureShapes({ kind, shapes });
}

export function draftPositionCount(draft: GeometryDraft): number {
	let count = 0;
	for (const shape of draft.shapes)
		for (const ring of shape) count += ring.length;
	return count;
}

export function clampRingRef(
	ref: GeometryRingRef,
	draft: GeometryDraft,
): GeometryRingRef {
	const shape = Math.min(ref.shape, draft.shapes.length - 1);
	const ring = Math.min(ref.ring, draft.shapes[shape].length - 1);
	return shape === ref.shape && ring === ref.ring ? ref : { shape, ring };
}

function updateRing(
	draft: GeometryDraft,
	ref: GeometryRingRef,
	update: (ring: GeometryRing) => GeometryRing,
): GeometryDraft {
	return {
		kind: draft.kind,
		shapes: draft.shapes.map((shape, s) =>
			s === ref.shape
				? shape.map((ring, r) => (r === ref.ring ? update(ring) : ring))
				: shape,
		),
	};
}

export function addDraftPosition(
	draft: GeometryDraft,
	ref: GeometryRingRef,
	position: GeometryPosition,
): GeometryDraft {
	const clamped = clampPosition(position);
	return updateRing(draft, ref, (ring) =>
		draftCapabilities(draft.kind).singlePosition ? [clamped] : [...ring, clamped],
	);
}

export function updateDraftPosition(
	draft: GeometryDraft,
	ref: GeometryRingRef,
	index: number,
	position: GeometryPosition,
): GeometryDraft {
	const clamped = clampPosition(position);
	return updateRing(draft, ref, (ring) =>
		ring.map((existing, i) => (i === index ? clamped : existing)),
	);
}

export function removeDraftPosition(
	draft: GeometryDraft,
	ref: GeometryRingRef,
	index: number,
): GeometryDraft {
	return updateRing(draft, ref, (ring) => ring.filter((_, i) => i !== index));
}

export function addDraftRing(
	draft: GeometryDraft,
	shapeIndex: number,
): GeometryDraft {
	if (!draftCapabilities(draft.kind).holes) return draft;
	return {
		kind: draft.kind,
		shapes: draft.shapes.map((shape, s) =>
			s === shapeIndex ? [...shape, []] : shape,
		),
	};
}

/** The exterior ring (index 0) cannot be removed; clear its positions instead. */
export function removeDraftRing(
	draft: GeometryDraft,
	ref: GeometryRingRef,
): GeometryDraft {
	if (ref.ring === 0) return draft;
	return {
		kind: draft.kind,
		shapes: draft.shapes.map((shape, s) =>
			s === ref.shape ? shape.filter((_, r) => r !== ref.ring) : shape,
		),
	};
}

export function addDraftShape(draft: GeometryDraft): GeometryDraft {
	if (!draftCapabilities(draft.kind).multiShape) return draft;
	return { kind: draft.kind, shapes: [...draft.shapes, [[]]] };
}

export function removeDraftShape(
	draft: GeometryDraft,
	shapeIndex: number,
): GeometryDraft {
	return ensureShapes({
		kind: draft.kind,
		shapes: draft.shapes.filter((_, s) => s !== shapeIndex),
	});
}
