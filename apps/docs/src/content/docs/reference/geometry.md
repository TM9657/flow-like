---
title: Geometry
description: Store locations and shapes, validate their types, and query spatial data.
---

A **Geometry** variable holds a GeoJSON geometry object. Geometry pins and tokens
use orange (`#F97316`). Choose Geometry in the variable editor, optionally select
a subtype, and enter GeoJSON. The editor validates the value before saving it and
previews valid shapes on a map.

```json
{"type":"Point","coordinates":[13.405,52.52]}
```

Coordinates are **longitude, latitude**, in WGS 84 degrees. The example places a
point in Berlin. Reversing those numbers describes a different location.

## Values and subtypes

Supported subtypes are Point, LineString, Polygon, MultiPoint, MultiLineString,
MultiPolygon, and GeometryCollection. A concrete subtype can connect to an input
that accepts any Geometry. To connect an unrestricted Geometry to a Point input,
use **Validate Point**. Each subtype has a corresponding validating cast node.

FlowScript spells these types `geometry` and `geometry<Point>`. Arrays, sets, and
maps contain Geometry values independently of their subtype. A MultiPoint is one
Geometry value; an array of Points is a container of separate values. Set equality
compares values, not spatial equivalence.

The current profile accepts exactly two finite coordinates per position, with
longitude between -180 and 180 and latitude between -90 and 90. Polygon rings must
be closed and contain at least four positions. Import and editing normalize ring
winding. Spatial operation nodes also check topology before computing a result.

Null represents an unset value. Required inputs reject it. Empty multi-geometries
and GeometryCollections are supported; empty Point, LineString, Polygon, and empty
members inside multi-geometries are rejected. Values are limited to 1 MiB of JSON,
32 levels of nesting, and 100,000 positions.

Feature and FeatureCollection wrappers are not Geometry values. Extract their
geometry first and retain their properties in a surrounding Struct. GeoJSON
conversion retains valid foreign members and bounding boxes. WKT and WKB preserve
the shape but cannot carry those extra JSON members. WKT imports assert WGS 84;
WKB imports accept two-dimensional values and an optional EPSG:4326 SRID. Other
coordinate systems and Z/M dimensions require conversion outside these nodes.

## Operations and existing flows

The Geometry catalog provides constructors, GeoJSON/WKT/WKB conversion, accessors,
predicates, intersection, centroid, convex hull, simplification, and measurements.
Planar operations use the longitude/latitude coordinate plane. Their distances
and lengths are degrees, and their areas are square degrees. They do not wrap
across the antimeridian.

Use the explicitly named meter and square-meter operations for WGS 84 geodesic
measurements. Geodesic distance accepts Points. Geodesic length includes line
segments and polygon ring perimeters. Geodesic area accepts polygons smaller than
half the Earth and subtracts holes.

Existing H3, routing, search, and map nodes retain their original pins. Adapter
nodes convert their coordinate, boundary, polygon, route, and location payloads.
Route and location adapters retain the surrounding result so properties remain
available to downstream nodes.

## Tables and SQL

Create a scalar **geometry** column in Data Studio or declare `"type":"geometry"`
in a table schema. The column stores WKB with GeoArrow WGS 84 metadata. Ordinary
Binary columns remain byte arrays, and Struct columns remain objects. Geometry
previews depend on the declared Arrow metadata.

Insert or upsert validated GeoJSON into a declared geometry column. Direct Arrow
inserts require compatible GeoArrow metadata and validate the values. Native
GeoArrow query output can be inserted into a declared WKB column. SQL INSERT into
geometry tables, geometry UPDATE assignments, and geometry expression columns
are currently rejected. Use validated insert/upsert operations instead.

Application SQL sessions expose spatial functions, including `ST_Intersects`,
`ST_Contains`, and `ST_Centroid`. To produce a Geometry result from WKT, use the
explicit WGS 84 import helper:

```sql
SELECT ST_Centroid(
  flow_geomfromtext('LINESTRING(10 20,20 30)')
) AS center;
```

`flow_geomfromtext` validates longitude/latitude coordinates and attaches WGS 84
metadata. It does not transform coordinates. `ST_GeomFromText` carries unknown CRS,
so its results require an explicit WGS 84 assertion before conversion to a flow
Geometry. Bind WKT parameters as strings, for example
`ST_Intersects(geom, flow_geomfromtext($1))`.

SQL spatial measurements are planar. Application SQL evaluates spatial predicates
above the Lance scan and applies limits after filtering. Direct Lance spatial
filters can use an RTree index. Do not infer index use from the presence of an
index alone; inspect the query plan for the query path being used.

## Client compatibility

Remote Geometry creation and execution are enabled by default. Geometry requires
board format version 2. Boards without a stored `format_version` use version 1;
loaders also infer version 2 for existing Geometry boards. Board format versions
are independent of app release numbers and compiled artifact versions.

Clients advertise their highest supported version with
`x-flow-like-board-format: 2`. A missing header means version 1. The endpoint
`/apps/{app}/board/capabilities` returns `{ "board_format_version": 2 }`, which
lets clients enable features supported by the backend.

Clients that cannot support a board receive `BOARD_FORMAT_UPGRADE_REQUIRED`
(HTTP 426) before receiving or changing it. Realtime sessions use rooms separated
by the negotiated board format. API, executor, and signaling deployments must
support version 2 before running Geometry flows. Existing Geometry boards retain
that requirement if a deployment is downgraded.
