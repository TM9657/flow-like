// geo — FlowScript node declarations (generated, do not edit).
// One `function` per catalog node, grouped by FlowScript namespace. Call a node as
// `ns::alias({ pin: value })`, or write `use ns::*` once at the top of a .flow file and
// call `alias({ pin: value })`. A `this: T` parameter marks the receiver pin: such a node
// is also a method on that value (`x.alias(...)`, remaining inputs positional or named).
// JSDoc tags carry the node type (`@node`), the receiver pin (`@receiver`) and the legacy
// camelCase spelling (`@alias`), which is still accepted.

declare namespace geo {
    // === Web/Geo/Map ===

    /**
     * Fetches a static map image for the given coordinates using OpenStreetMap tiles. Returns a satellite/standard map image centered on the location.
     * @node geo_get_map_image @alias geoGetMapImage
     * @param coordinate — The geographic coordinate (latitude, longitude) to center the map on
     * @param zoom (optional) — Map zoom level (1-19). Higher values show more detail. Default: 15
     * @param width (optional) — Image width in pixels. Default: 512
     * @param height (optional) — Image height in pixels. Default: 512
     * @param style (optional) — Map style to use
     * @returns image — The fetched map image
     * @impure has side effects / drives control flow
     */
    function getMapImage({ coordinate: Struct, zoom?: int, width?: int, height?: int, style?: string }): Struct;

    // === Web/Geo/Routing ===

    /**
     * Snaps noisy GPS traces to the road network using OSRM map matching.
     * @node geo_osrm_match_trace @alias geoOsrmMatchTrace
     * @param coordinates (optional) — Ordered GPS coordinates to match
     * @param profile (optional) — Transportation mode: Car, Bike, or Foot
     * @param timestamps (optional) — Optional UNIX timestamps for each coordinate (seconds)
     * @param radiuses (optional) — Optional search radiuses in meters for each coordinate
     * @param gaps (optional) — How to handle gaps: split or ignore
     * @param tidy (optional) — Simplify the matched geometry
     * @param baseUrl (optional) — OSRM server base URL
     * @returns matchings — Matched routes for the trace
     * @returns primaryMatching — Primary matched route
     * @returns tracepoints — Tracepoints mapped to the road network
     * @impure has side effects / drives control flow
     */
    function osrmMatchTrace({ coordinates?: Struct[], profile?: Struct, timestamps?: int[], radiuses?: float[], gaps?: string, tidy?: bool, baseUrl?: string }): { matchings: Struct[], primaryMatching: Struct, tracepoints: Struct[] };

    /**
     * Finds the nearest routable point(s) to a coordinate using OSRM.
     * @node geo_osrm_nearest @alias geoOsrmNearest
     * @param coordinate — The coordinate to snap to the road network
     * @param profile (optional) — Transportation mode: Car, Bike, or Foot
     * @param number (optional) — Maximum number of nearest points to return (1-50)
     * @param baseUrl (optional) — OSRM server base URL
     * @returns nearest — The closest routable point
     * @returns waypoints — List of nearest routable points
     * @impure has side effects / drives control flow
     */
    function osrmNearest({ coordinate: Struct, profile?: Struct, number?: int, baseUrl?: string }): { nearest: Struct, waypoints: Struct[] };

    /**
     * Computes travel time and distance matrices between coordinates using OSRM.
     * @node geo_osrm_table @alias geoOsrmTable
     * @param coordinates (optional) — List of coordinates to include in the matrix
     * @param profile (optional) — Transportation mode: Car, Bike, or Foot
     * @param sources (optional) — Optional indices of source coordinates
     * @param destinations (optional) — Optional indices of destination coordinates
     * @param includeDurations (optional) — Return travel time matrix
     * @param includeDistances (optional) — Return travel distance matrix
     * @param baseUrl (optional) — OSRM server base URL
     * @returns durations — Matrix of travel times in seconds
     * @returns distances — Matrix of travel distances in meters
     * @returns result — Matrix result containing durations and distances
     * @impure has side effects / drives control flow
     */
    function osrmTable({ coordinates?: Struct[], profile?: string, sources?: int[], destinations?: int[], includeDurations?: bool, includeDistances?: bool, baseUrl?: string }): { durations: Struct[], distances: Struct[], result: Struct };

    /**
     * Fetches vector map tiles (MVT) from an OSRM server.
     * @node geo_osrm_tile @alias geoOsrmTile
     * @param profile (optional) — Transportation mode: Car, Bike, or Foot
     * @param z (optional) — Tile zoom level
     * @param x (optional) — Tile X coordinate
     * @param y (optional) — Tile Y coordinate
     * @param path — Destination path for the MVT tile
     * @param baseUrl (optional) — OSRM server base URL
     * @returns tilePath — Stored tile path
     * @returns contentType — Content type returned by the server
     * @impure has side effects / drives control flow
     */
    function osrmTile({ profile?: Struct, z?: int, x?: int, y?: int, path: Struct, baseUrl?: string }): { tilePath: Struct, contentType: string };

    /**
     * Plans the shortest round trip through multiple coordinates using OSRM.
     * @node geo_osrm_trip @alias geoOsrmTrip
     * @param coordinates (optional) — Ordered coordinates for the trip
     * @param profile (optional) — Transportation mode: Car, Bike, or Foot
     * @param roundtrip (optional) — Return to the starting point
     * @param source (optional) — Source location: any, first, or last
     * @param destination (optional) — Destination location: any, first, or last
     * @param baseUrl (optional) — OSRM server base URL
     * @returns trip — Primary trip result
     * @returns trips — All trip results returned by OSRM
     * @returns waypoints — Optimized trip waypoints
     * @returns distance — Total trip distance in meters
     * @returns duration — Total trip duration in seconds
     * @returns geometry — Trip geometry as array of coordinates
     * @impure has side effects / drives control flow
     */
    function osrmTrip({ coordinates?: Struct[], profile?: Struct, roundtrip?: bool, source?: string, destination?: string, baseUrl?: string }): { trip: Struct, trips: Struct[], waypoints: Struct[], distance: float, duration: float, geometry: Struct[] };

    /**
     * Plans a route between two points using the OSRM routing service. Returns turn-by-turn directions, distance, and duration.
     * @node geo_plan_route @alias geoPlanRoute
     * @param start — Starting coordinate for the route
     * @param end — Ending coordinate for the route
     * @param waypoints (optional) — Optional intermediate waypoints to pass through
     * @param profile (optional) — Transportation mode: Car, Bike, or Foot
     * @param alternatives (optional) — Request alternative routes
     * @returns route — The primary calculated route
     * @returns alternativesOut — Alternative routes if requested
     * @returns distance — Total route distance in meters
     * @returns duration — Estimated travel time in seconds
     * @returns geometry — Route geometry as array of coordinates
     * @impure has side effects / drives control flow
     */
    function planRoute({ start: Struct, end: Struct, waypoints?: Struct, profile?: string, alternatives?: bool }): { route: Struct, alternativesOut: Struct, distance: float, duration: float, geometry: Struct };

    // === Web/Geo/Search ===

    /**
     * Converts geographic coordinates to a human-readable address using the Nominatim service (OpenStreetMap).
     * @node geo_reverse_geocode @alias geoReverseGeocode
     * @param coordinate — The geographic coordinate (latitude, longitude) to look up
     * @param zoom (optional) — Level of detail for the address (0-18). Higher = more specific. Default: 18
     * @returns result — The reverse geocoding result with address details
     * @returns displayName — The full formatted address string
     * @impure has side effects / drives control flow
     */
    function reverseGeocode({ coordinate: Struct, zoom?: int }): { result: Struct, displayName: string };

    /**
     * Searches for a location by name or address using the Nominatim geocoding service (OpenStreetMap). Returns matching locations with coordinates.
     * @node geo_search_location @alias geoSearchLocation
     * @param query (optional) — The search query (address, place name, etc.)
     * @param limit (optional) — Maximum number of results to return. Default: 5
     * @param countryCodes (optional) — Optional comma-separated list of country codes to limit search (e.g., 'de,at,ch')
     * @returns results — Array of search results with coordinates
     * @returns firstResult — The first/best matching result (if any)
     * @impure has side effects / drives control flow
     */
    function searchLocation({ query?: string, limit?: int, countryCodes?: string }): { results: Struct[], firstResult: Struct };
}

declare namespace geometry {
    // === Web/Geo/Geometry ===

    /**
     * Returns minimum and maximum longitude and latitude using a planar coordinate envelope. Empty geometries have no bounds.
     * @node geometry_bounds @receiver geometry @alias geometryBounds
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.bounds(...)`)
     * @returns minLongitude — Coordinate in degrees
     * @returns minLatitude — Coordinate in degrees
     * @returns maxLongitude — Coordinate in degrees
     * @returns maxLatitude — Coordinate in degrees
     */
    function bounds(this: geometry, { geometry: geometry }): { minLongitude: float, minLatitude: float, maxLongitude: float, maxLatitude: float };

    /**
     * Validates a geometry as GeometryCollection and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_geometry_collection @receiver geometry @alias geometryCastGeometryCollection
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castGeometryCollection(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castGeometryCollection(this: geometry, { geometry: geometry }): geometry<GeometryCollection>;

    /**
     * Validates a geometry as LineString and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_line_string @receiver geometry @alias geometryCastLineString
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castLineString(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castLineString(this: geometry, { geometry: geometry }): geometry<LineString>;

    /**
     * Validates a geometry as MultiLineString and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_multi_line_string @receiver geometry @alias geometryCastMultiLineString
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castMultiLineString(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castMultiLineString(this: geometry, { geometry: geometry }): geometry<MultiLineString>;

    /**
     * Validates a geometry as MultiPoint and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_multi_point @receiver geometry @alias geometryCastMultiPoint
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castMultiPoint(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castMultiPoint(this: geometry, { geometry: geometry }): geometry<MultiPoint>;

    /**
     * Validates a geometry as MultiPolygon and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_multi_polygon @receiver geometry @alias geometryCastMultiPolygon
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castMultiPolygon(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castMultiPolygon(this: geometry, { geometry: geometry }): geometry<MultiPolygon>;

    /**
     * Validates a geometry as Point and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_point @receiver geometry @alias geometryCastPoint
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castPoint(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castPoint(this: geometry, { geometry: geometry }): geometry<Point>;

    /**
     * Validates a geometry as Polygon and returns it with an explicit subtype. Incompatible values fail the node.
     * @node geometry_cast_polygon @receiver geometry @alias geometryCastPolygon
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.castPolygon(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function castPolygon(this: geometry, { geometry: geometry }): geometry<Polygon>;

    /**
     * Computes the centroid in the longitude/latitude coordinate plane. Empty geometries have no centroid.
     * @node geometry_centroid @receiver geometry @alias geometryCentroid
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.centroid(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function centroid(this: geometry, { geometry: geometry }): geometry<Point>;

    /**
     * Tests whether geometry A contains B in the longitude/latitude coordinate plane. A point on a polygon boundary is not contained.
     * @node geometry_contains @receiver a @alias geometryContains
     * @param a — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.contains(...)`)
     * @param b — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns result — Planar predicate result
     */
    function contains(this: geometry, { a: geometry, b: geometry }): bool;

    /**
     * Computes a polygon hull in the longitude/latitude coordinate plane. Fails when the input cannot form a valid polygon with at least three non-collinear positions.
     * @node geometry_convex_hull @receiver geometry @alias geometryConvexHull
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.convexHull(...)`)
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function convexHull(this: geometry, { geometry: geometry }): geometry<Polygon>;

    /**
     * Parses a GeoJSON geometry object, validates its two-dimensional WGS 84 coordinates and normalizes ring winding. Retains bbox and foreign members. Feature wrappers require extraction.
     * @node geometry_from_geojson @alias geometryFromGeojson
     * @param text — text
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function fromGeoJson({ text: string }): geometry;

    /**
     * Converts the coordinate vector emitted by H3 Cell Boundary into a Polygon. Closes the ring and validates topology.
     * @node geometry_from_legacy_boundary @alias geometryFromLegacyBoundary
     * @param boundary — boundary
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function fromLegacyBoundary({ boundary: Struct }): geometry<Polygon>;

    /**
     * Converts the existing GeoCoordinate latitude/longitude object to a Geometry Point without swapping the axes.
     * @node geometry_from_legacy_coordinate @alias geometryFromLegacyCoordinate
     * @param coordinate — coordinate
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function fromLegacyCoordinate({ coordinate: Struct }): geometry<Point>;

    /**
     * Extracts a Point from a search result or waypoint coordinate. Returns the original rich location wrapper unchanged.
     * @node geometry_from_legacy_location @alias geometryFromLegacyLocation
     * @param location — location
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     * @returns locationOut — Original location wrapper
     */
    function fromLegacyLocation({ location: Struct }): { geometryOut: geometry<Point>, locationOut: Struct };

    /**
     * Converts the existing H3 polygon vector, preserving exterior and interior rings and closing each ring.
     * @node geometry_from_legacy_polygons @alias geometryFromLegacyPolygons
     * @param polygons — polygons
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function fromLegacyPolygons({ polygons: Struct }): geometry<MultiPolygon>;

    /**
     * Extracts a LineString from RouteResult.geometry.points or RouteGeometry.points. Returns the original wrapper unchanged.
     * @node geometry_from_legacy_route @alias geometryFromLegacyRoute
     * @param route — route
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     * @returns routeOut — Original route wrapper
     */
    function fromLegacyRoute({ route: Struct }): { geometryOut: geometry<LineString>, routeOut: Struct };

    /**
     * Parses two-dimensional WKB bytes. Calling this node asserts WGS 84 longitude/latitude. Unsupported dimensions, SRIDs and out-of-range coordinates are rejected.
     * @node geometry_from_wkb @alias geometryFromWkb
     * @param bytes — bytes
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function fromWkb({ bytes: bytes[] }): geometry;

    /**
     * Parses two-dimensional WKT. Calling this node asserts coordinates are WGS 84 longitude/latitude; it does not transform a projected CRS. Empty scalar geometries are rejected.
     * @node geometry_from_wkt @alias geometryFromWkt
     * @param text — text
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function fromWkt({ text: string }): geometry;

    /**
     * Computes WGS 84 ellipsoidal Polygon or MultiPolygon area in square meters, subtracting holes. Each polygon must describe a region smaller than half the Earth.
     * @node geometry_geodesic_area @receiver geometry @alias geometryGeodesicArea
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.geodesicArea(...)`)
     * @returns area — Area in square meters
     */
    function geodesicArea(this: geometry, { geometry: geometry }): float;

    /**
     * Computes the WGS 84 ellipsoidal geodesic distance between two Points in meters, including antimeridian crossings.
     * @node geometry_geodesic_distance @receiver a @alias geometryGeodesicDistance
     * @param a — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.geodesicDistance(...)`)
     * @param b — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns distance — Distance in meters
     */
    function geodesicDistance(this: geometry<Point>, { a: geometry<Point>, b: geometry<Point> }): float;

    /**
     * Sums WGS 84 ellipsoidal geodesic segment lengths in meters, including polygon exterior and interior ring perimeters. Points contribute zero.
     * @node geometry_geodesic_length @receiver geometry @alias geometryGeodesicLength
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.geodesicLength(...)`)
     * @returns length — Length in meters
     */
    function geodesicLength(this: geometry, { geometry: geometry }): float;

    /**
     * Intersects Polygon or MultiPolygon inputs in the longitude/latitude coordinate plane. Returns a MultiPolygon, which can be empty.
     * @node geometry_intersection @receiver a @alias geometryIntersection
     * @param a — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.intersection(...)`)
     * @param b — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function intersection(this: geometry, { a: geometry, b: geometry }): geometry<MultiPolygon>;

    /**
     * Tests whether geometries share any point in the longitude/latitude coordinate plane, including boundary touches.
     * @node geometry_intersects @receiver a @alias geometryIntersects
     * @param a — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.intersects(...)`)
     * @param b — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns result — Planar predicate result
     */
    function intersects(this: geometry, { a: geometry, b: geometry }): bool;

    /**
     * Creates a WGS 84 Point from longitude and latitude. Both coordinates must be finite and within geographic bounds.
     * @node geometry_make_point @alias geometryMakePoint
     * @param longitude — longitude
     * @param latitude — latitude
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function makePoint({ longitude: float, latitude: float }): geometry<Point>;

    /**
     * Counts coordinate positions, including closing polygon positions and recursive collection members.
     * @node geometry_num_points @receiver geometry @alias geometryNumPoints
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.numPoints(...)`)
     * @returns count — Number of positions, including closing ring positions
     */
    function numPoints(this: geometry, { geometry: geometry }): int;

    /**
     * Computes Polygon or MultiPolygon area in square coordinate degrees, subtracting holes. This is a planar measurement.
     * @node geometry_planar_area @receiver geometry @alias geometryPlanarArea
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.planarArea(...)`)
     * @returns area — Planar area in square coordinate degrees
     */
    function planarArea(this: geometry, { geometry: geometry }): float;

    /**
     * Computes the shortest planar distance in coordinate degrees. This longitude/latitude plane does not wrap at the antimeridian. Empty inputs are rejected.
     * @node geometry_planar_distance @receiver a @alias geometryPlanarDistance
     * @param a — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.planarDistance(...)`)
     * @param b — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns distance — Planar distance in coordinate degrees
     */
    function planarDistance(this: geometry, { a: geometry, b: geometry }): float;

    /**
     * Sums line lengths and polygon ring perimeters in planar coordinate degrees. Points contribute zero.
     * @node geometry_planar_length @receiver geometry @alias geometryPlanarLength
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.planarLength(...)`)
     * @returns length — Planar length in coordinate degrees
     */
    function planarLength(this: geometry, { geometry: geometry }): float;

    /**
     * Simplifies lines and polygon rings with a nonnegative tolerance in coordinate degrees. Validates the result and rejects a topology-breaking result.
     * @node geometry_simplify @receiver geometry @alias geometrySimplify
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.simplify(...)`)
     * @param tolerance — tolerance
     * @returns geometryOut — Validated WGS 84 GeoJSON geometry
     */
    function simplify(this: geometry, { geometry: geometry, tolerance: float }): geometry;

    /**
     * Writes a validated geometry as GeoJSON text, retaining bbox and foreign members and normalizing ring winding.
     * @node geometry_to_geojson @receiver geometry @alias geometryToGeojson
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.toGeoJson(...)`)
     * @returns text — Serialized geometry
     */
    function toGeoJson(this: geometry, { geometry: geometry }): string;

    /**
     * Converts a Geometry Point into the existing GeoCoordinate shape used by H3, routing, search and map nodes.
     * @node geometry_to_legacy_coordinate @receiver geometry @alias geometryToLegacyCoordinate
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.toLegacyCoordinate(...)`)
     * @returns coordinate — Legacy latitude/longitude object
     */
    function toLegacyCoordinate(this: geometry<Point>, { geometry: geometry<Point> }): Struct;

    /**
     * Updates only the points of an existing RouteResult or RouteGeometry with a LineString, retaining route metadata and other fields.
     * @node geometry_to_legacy_route @alias geometryToLegacyRoute
     * @param route — route
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns routeOut — Route with updated points and retained metadata
     */
    function toLegacyRoute({ route: Struct, geometry: geometry<LineString> }): Struct;

    /**
     * Writes two-dimensional WKB bytes. WKB omits GeoJSON bbox and foreign members; keep application properties in a surrounding Struct.
     * @node geometry_to_wkb @receiver geometry @alias geometryToWkb
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.toWkb(...)`)
     * @returns bytes — WKB byte sequence
     */
    function toWkb(this: geometry, { geometry: geometry }): bytes[];

    /**
     * Writes two-dimensional WKT. WKT omits GeoJSON bbox and foreign members; keep application properties in a surrounding Struct.
     * @node geometry_to_wkt @receiver geometry @alias geometryToWkt
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.toWkt(...)`)
     * @returns text — Serialized geometry
     */
    function toWkt(this: geometry, { geometry: geometry }): string;

    /**
     * Returns the GeoJSON geometry type name.
     * @node geometry_type @receiver geometry @alias geometryType
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.type(...)`)
     * @returns type — GeoJSON type name
     */
    function type(this: geometry, { geometry: geometry }): string;

    /**
     * Tests whether geometry B contains A in the longitude/latitude coordinate plane.
     * @node geometry_within @receiver a @alias geometryWithin
     * @param a — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.within(...)`)
     * @param b — Validated WGS 84 longitude/latitude GeoJSON geometry
     * @returns result — Planar predicate result
     */
    function within(this: geometry, { a: geometry, b: geometry }): bool;

    /**
     * Returns the Point x coordinate, longitude in degrees.
     * @node geometry_x @receiver geometry @alias geometryX
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.x(...)`)
     * @returns value — Coordinate in degrees
     */
    function x(this: geometry<Point>, { geometry: geometry<Point> }): float;

    /**
     * Returns the Point y coordinate, latitude in degrees.
     * @node geometry_y @receiver geometry @alias geometryY
     * @param geometry — Validated WGS 84 longitude/latitude GeoJSON geometry (receiver: `this` in `x.y(...)`)
     * @returns value — Coordinate in degrees
     */
    function y(this: geometry<Point>, { geometry: geometry<Point> }): float;
}

declare namespace h3 {
    // === Web/Geo/H3 ===

    /**
     * Calculates the area of an H3 cell in the specified unit.
     * @node h3_cell_area @alias h3CellArea
     * @param cell (optional) — H3 cell index
     * @param unit (optional) — Area unit for the result
     * @returns area — Area of the cell in the specified unit
     * @returns resolution — Resolution of the cell
     */
    function cellArea({ cell?: string, unit?: Struct }): { area: float, resolution: int };

    /**
     * Returns the polygon boundary (vertices) of an H3 cell. Useful for visualization and geospatial operations.
     * @node h3_cell_to_boundary @alias h3CellToBoundary
     * @param cell (optional) — H3 cell index as a hexadecimal string
     * @returns boundary — Array of coordinates representing the cell boundary (closed polygon)
     * @returns vertexCount — Number of vertices (typically 6 for hexagons, 5 for pentagons)
     */
    function cellToBoundary({ cell?: string }): { boundary: Struct, vertexCount: int };

    /**
     * Returns all child cells at a finer resolution that fit within the given cell.
     * @node h3_cell_to_children @alias h3CellToChildren
     * @param cell (optional) — H3 cell index
     * @param childResolution (optional) — Target resolution for children (must be higher than cell's resolution)
     * @returns children — Array of child H3 cell indices
     * @returns count — Number of child cells
     */
    function cellToChildren({ cell?: string, childResolution?: int }): { children: string[], count: int };

    /**
     * Converts an H3 cell index to the geographic coordinate of its center point.
     * @node h3_cell_to_latlng @alias h3CellToLatlng
     * @param cell (optional) — H3 cell index as a hexadecimal string
     * @returns coordinate — The center coordinate of the H3 cell
     */
    function cellToLatlng({ cell?: string }): Struct;

    /**
     * Returns the parent cell at a coarser resolution. The parent contains the given cell.
     * @node h3_cell_to_parent @alias h3CellToParent
     * @param cell (optional) — H3 cell index
     * @param parentResolution (optional) — Target resolution for the parent (must be lower than cell's resolution)
     * @returns parent — Parent H3 cell index at the specified resolution
     * @returns originalResolution — Resolution of the input cell
     */
    function cellToParent({ cell?: string, parentResolution?: int }): { parent: string, originalResolution: int };

    /**
     * Converts a set of H3 cells to polygon boundaries. Returns the outline(s) of the cell set, merging adjacent cells.
     * @node h3_cells_to_multi_polygon @alias h3CellsToMultiPolygon
     * @param cells (optional) — Array of H3 cell indices
     * @returns polygons — Array of polygons representing the merged cell boundaries
     * @returns polygonCount — Number of separate polygons (disconnected regions)
     */
    function cellsToMultiPolygon({ cells?: string[] }): { polygons: Struct, polygonCount: int };

    /**
     * Compacts a set of H3 cells by replacing groups of cells with their parent when all children are present. Reduces the number of cells while covering the same area.
     * @node h3_compact_cells @alias h3CompactCells
     * @param cells (optional) — Array of H3 cell indices to compact
     * @returns compacted — Array of compacted H3 cell indices (may contain mixed resolutions)
     * @returns originalCount — Number of input cells
     * @returns compactedCount — Number of cells after compaction
     */
    function compactCells({ cells?: string[] }): { compacted: string[], originalCount: int, compactedCount: int };

    /**
     * Returns the average edge length of H3 cells at a given resolution.
     * @node h3_edge_length @alias h3EdgeLength
     * @param resolution (optional) — H3 resolution (0-15)
     * @param unit (optional) — Length unit for the result
     * @returns edgeLength — Average edge length at this resolution
     * @returns cellCount — Total number of cells at this resolution covering Earth
     */
    function edgeLength({ resolution?: int, unit?: Struct }): { edgeLength: float, cellCount: int };

    /**
     * Returns all H3 cells within k steps of the origin cell (a filled disk of hexagons). Useful for proximity searches and area coverage.
     * @node h3_grid_disk @alias h3GridDisk
     * @param cell (optional) — Origin H3 cell index
     * @param k (optional) — Number of rings around the origin (0 = just the origin cell)
     * @returns cells — Array of H3 cell indices in the disk
     * @returns count — Number of cells in the disk
     */
    function gridDisk({ cell?: string, k?: int }): { cells: string[], count: int };

    /**
     * Calculates the grid distance (number of steps) between two H3 cells. Both cells must be at the same resolution.
     * @node h3_grid_distance @alias h3GridDistance
     * @param cellA (optional) — First H3 cell index
     * @param cellB (optional) — Second H3 cell index
     * @returns distance — Grid distance (number of hexagon steps) between the cells
     */
    function gridDistance({ cellA?: string, cellB?: string }): int;

    /**
     * Finds a path of H3 cells between two cells. Returns all cells along the shortest path. Both cells must be at the same resolution.
     * @node h3_grid_path @alias h3GridPath
     * @param cellA (optional) — Starting H3 cell index
     * @param cellB (optional) — Ending H3 cell index
     * @returns path — Array of H3 cell indices along the path (including start and end)
     * @returns length — Number of cells in the path
     */
    function gridPath({ cellA?: string, cellB?: string }): { path: string[], length: int };

    /**
     * Converts a geographic coordinate to an H3 cell index at the specified resolution. H3 is a hierarchical hexagonal grid system.
     * @node h3_latlng_to_cell @alias h3LatlngToCell
     * @param coordinate — The geographic coordinate (latitude, longitude)
     * @param resolution (optional) — H3 resolution (0-15). Higher = smaller cells. 0 = ~4,357,449 km², 15 = ~0.9 m²
     * @returns cell — H3 cell index as a hexadecimal string
     */
    function latlngToCell({ coordinate: Struct, resolution?: int }): string;
}
