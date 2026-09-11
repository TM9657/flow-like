//! Geometry-native adapters for GeoJSON wrappers, encoded paths and spatial indexes.

use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::{Pin, PinType, ValueType},
    variable::VariableType,
};
use flow_like_types::{
    async_trait,
    geometry::{GeometryKind, marker},
    json::json,
};

#[cfg(feature = "execute")]
use flow_like_geometry::{from_geo, to_geo};
#[cfg(feature = "execute")]
use flow_like_types::{Result, Value, anyhow, bail, geometry::canonicalize_geometry, json::Map};

#[derive(Clone, Copy, Debug)]
enum Operation {
    FeatureGeometry,
    MakeFeature,
    FeatureCollectionGeometries,
    MakeFeatureCollection,
    EncodePolyline,
    DecodePolyline,
    PointToH3Cell,
    H3CellBoundary,
    H3CellsToGeometry,
    PolygonToH3Cells,
    EncodeGeohash,
    DecodeGeohash,
}

fn geometry_input<'a>(
    node: &'a mut Node,
    name: &str,
    description: &str,
    kind: Option<GeometryKind>,
) -> &'a mut Pin {
    let pin = node.add_input_pin(name, name, description, VariableType::Geometry);
    pin.schema = kind.map(|kind| marker(kind).to_string());
    pin
}

fn geometry_output<'a>(
    node: &'a mut Node,
    name: &str,
    description: &str,
    kind: Option<GeometryKind>,
) -> &'a mut Pin {
    let pin = node.add_output_pin(name, name, description, VariableType::Geometry);
    pin.schema = kind.map(|kind| marker(kind).to_string());
    pin
}

fn data_input<'a>(
    node: &'a mut Node,
    name: &str,
    description: &str,
    data_type: VariableType,
) -> &'a mut Pin {
    node.add_input_pin(name, name, description, data_type)
}

fn data_output<'a>(
    node: &'a mut Node,
    name: &str,
    description: &str,
    data_type: VariableType,
) -> &'a mut Pin {
    node.add_output_pin(name, name, description, data_type)
}

fn definition(operation: Operation) -> Node {
    use Operation::*;
    let (id, alias, title, description) = match operation {
        FeatureGeometry => (
            "geometry_feature_geometry",
            "featureGeometry",
            "GeoJSON Feature Geometry",
            "Extracts and validates the non-null Geometry and properties from a GeoJSON Feature.",
        ),
        MakeFeature => (
            "geometry_make_feature",
            "makeFeature",
            "Make GeoJSON Feature",
            "Wraps a Geometry and properties in a GeoJSON Feature. A non-empty id is included as the Feature id.",
        ),
        FeatureCollectionGeometries => (
            "geometry_feature_collection_geometries",
            "featureCollectionGeometries",
            "GeoJSON FeatureCollection Geometries",
            "Extracts validated geometries and properties from a GeoJSON FeatureCollection. Null Feature geometries are rejected because Geometry values cannot be null.",
        ),
        MakeFeatureCollection => (
            "geometry_make_feature_collection",
            "makeFeatureCollection",
            "Make GeoJSON FeatureCollection",
            "Creates a GeoJSON FeatureCollection from Geometry values and an optional matching array of property objects.",
        ),
        EncodePolyline => (
            "geometry_encode_polyline",
            "encodePolyline",
            "Encode Polyline",
            "Encodes a LineString with the Google encoded polyline algorithm. Precision is the number of decimal coordinate digits.",
        ),
        DecodePolyline => (
            "geometry_decode_polyline",
            "decodePolyline",
            "Decode Polyline",
            "Decodes a Google encoded polyline into a validated WGS 84 LineString.",
        ),
        PointToH3Cell => (
            "geometry_point_to_h3_cell",
            "pointToH3Cell",
            "Point to H3 Cell",
            "Converts a Geometry Point to an H3 cell index at resolution 0 through 15.",
        ),
        H3CellBoundary => (
            "geometry_h3_cell_boundary",
            "h3CellBoundary",
            "H3 Cell Boundary Geometry",
            "Returns an H3 cell boundary directly as a Polygon Geometry.",
        ),
        H3CellsToGeometry => (
            "geometry_h3_cells_to_geometry",
            "h3CellsToGeometry",
            "H3 Cells to Geometry",
            "Dissolves unique H3 cells at one resolution into a MultiPolygon Geometry.",
        ),
        PolygonToH3Cells => (
            "geometry_polygon_to_h3_cells",
            "polygonToH3Cells",
            "Polygon to H3 Cells",
            "Covers a Polygon or MultiPolygon with H3 cells. The containment mode controls whether centroids, complete boundaries or intersections qualify.",
        ),
        EncodeGeohash => (
            "geometry_encode_geohash",
            "encodeGeohash",
            "Point to Geohash",
            "Encodes a Geometry Point as a geohash with 1 through 12 characters.",
        ),
        DecodeGeohash => (
            "geometry_decode_geohash",
            "decodeGeohash",
            "Decode Geohash",
            "Decodes a geohash into its center Point and rectangular bounding Polygon.",
        ),
    };

    let mut node = Node::new(id, title, description, "Web/Geo/Geometry");
    node.set_flowscript_name("geometry", alias);
    node.add_icon("/flow/icons/map.svg");

    match operation {
        FeatureGeometry => {
            data_input(
                &mut node,
                "feature",
                "GeoJSON Feature object",
                VariableType::Struct,
            );
            geometry_output(&mut node, "geometry_out", "Extracted Geometry", None);
            data_output(
                &mut node,
                "properties",
                "Feature properties, or an empty object when omitted or null",
                VariableType::Struct,
            );
        }
        MakeFeature => {
            geometry_input(&mut node, "geometry", "Feature Geometry", None);
            data_input(
                &mut node,
                "properties",
                "Feature properties",
                VariableType::Struct,
            )
            .set_default_value(Some(json!({})));
            data_input(
                &mut node,
                "id",
                "Optional string Feature id",
                VariableType::String,
            )
            .set_default_value(Some(json!("")));
            data_output(
                &mut node,
                "feature",
                "GeoJSON Feature object",
                VariableType::Struct,
            );
        }
        FeatureCollectionGeometries => {
            data_input(
                &mut node,
                "feature_collection",
                "GeoJSON FeatureCollection object",
                VariableType::Struct,
            );
            geometry_output(
                &mut node,
                "geometry_out",
                "GeometryCollection containing each Feature Geometry",
                Some(GeometryKind::GeometryCollection),
            );
            data_output(
                &mut node,
                "properties",
                "Property objects in Feature order",
                VariableType::Struct,
            )
            .set_value_type(ValueType::Array);
        }
        MakeFeatureCollection => {
            geometry_input(&mut node, "geometries", "Feature geometries", None)
                .set_value_type(ValueType::Array)
                .set_default_value(Some(json!([])));
            data_input(
                &mut node,
                "properties",
                "Property objects, either empty or one per Geometry",
                VariableType::Struct,
            )
            .set_value_type(ValueType::Array)
            .set_default_value(Some(json!([])));
            data_output(
                &mut node,
                "feature_collection",
                "GeoJSON FeatureCollection object",
                VariableType::Struct,
            );
        }
        EncodePolyline => {
            geometry_input(
                &mut node,
                "geometry",
                "LineString to encode",
                Some(GeometryKind::LineString),
            );
            data_input(
                &mut node,
                "precision",
                "Decimal coordinate digits, from 0 through 10",
                VariableType::Integer,
            )
            .set_default_value(Some(json!(5)));
            data_output(
                &mut node,
                "polyline",
                "Encoded polyline text",
                VariableType::String,
            );
        }
        DecodePolyline => {
            data_input(
                &mut node,
                "polyline",
                "Encoded polyline text",
                VariableType::String,
            );
            data_input(
                &mut node,
                "precision",
                "Decimal coordinate digits, from 0 through 10",
                VariableType::Integer,
            )
            .set_default_value(Some(json!(5)));
            geometry_output(
                &mut node,
                "geometry_out",
                "Decoded LineString",
                Some(GeometryKind::LineString),
            );
        }
        PointToH3Cell => {
            geometry_input(
                &mut node,
                "geometry",
                "Point to index",
                Some(GeometryKind::Point),
            );
            data_input(
                &mut node,
                "resolution",
                "H3 resolution from 0 through 15",
                VariableType::Integer,
            )
            .set_default_value(Some(json!(9)));
            data_output(&mut node, "cell", "H3 cell index", VariableType::String);
        }
        H3CellBoundary => {
            data_input(&mut node, "cell", "H3 cell index", VariableType::String);
            geometry_output(
                &mut node,
                "geometry_out",
                "Cell boundary Polygon",
                Some(GeometryKind::Polygon),
            );
        }
        H3CellsToGeometry => {
            data_input(
                &mut node,
                "cells",
                "Unique H3 cells at one resolution",
                VariableType::String,
            )
            .set_value_type(ValueType::Array)
            .set_default_value(Some(json!([])));
            geometry_output(
                &mut node,
                "geometry_out",
                "Dissolved MultiPolygon",
                Some(GeometryKind::MultiPolygon),
            );
        }
        PolygonToH3Cells => {
            geometry_input(
                &mut node,
                "geometry",
                "Polygon or MultiPolygon to cover",
                None,
            );
            data_input(
                &mut node,
                "resolution",
                "H3 resolution from 0 through 15",
                VariableType::Integer,
            )
            .set_default_value(Some(json!(9)));
            data_input(
                &mut node,
                "containment",
                "One of centroid, contains-boundary, intersects-boundary or covers",
                VariableType::String,
            )
            .set_default_value(Some(json!("centroid")));
            data_input(
                &mut node,
                "max_cells",
                "Maximum cells to emit, from 1 through 1000000",
                VariableType::Integer,
            )
            .set_default_value(Some(json!(100_000)));
            data_output(&mut node, "cells", "H3 cell indexes", VariableType::String)
                .set_value_type(ValueType::Array);
        }
        EncodeGeohash => {
            geometry_input(
                &mut node,
                "geometry",
                "Point to encode",
                Some(GeometryKind::Point),
            );
            data_input(
                &mut node,
                "precision",
                "Geohash length from 1 through 12",
                VariableType::Integer,
            )
            .set_default_value(Some(json!(9)));
            data_output(
                &mut node,
                "geohash",
                "Encoded geohash",
                VariableType::String,
            );
        }
        DecodeGeohash => {
            data_input(&mut node, "geohash", "Geohash text", VariableType::String);
            geometry_output(
                &mut node,
                "center",
                "Geohash cell center",
                Some(GeometryKind::Point),
            );
            geometry_output(
                &mut node,
                "bounds",
                "Geohash cell bounds",
                Some(GeometryKind::Polygon),
            );
        }
    }

    if let Some(receiver) = node
        .pins
        .values()
        .filter(|pin| pin.pin_type == PinType::Input)
        .min_by_key(|pin| pin.index)
        .filter(|pin| pin.data_type == VariableType::Geometry)
        .map(|pin| pin.name.clone())
    {
        node.set_receiver(&receiver);
    }
    node
}

#[cfg(feature = "execute")]
fn input<'a>(inputs: &'a Value, name: &str) -> Result<&'a Value> {
    inputs
        .get(name)
        .ok_or_else(|| anyhow!("Missing input {name}"))
}

#[cfg(feature = "execute")]
fn integer(inputs: &Value, name: &str, minimum: i64, maximum: i64) -> Result<i64> {
    let value = input(inputs, name)?
        .as_i64()
        .ok_or_else(|| anyhow!("{name} must be an integer"))?;
    if !(minimum..=maximum).contains(&value) {
        bail!("{name} must be from {minimum} through {maximum}");
    }
    Ok(value)
}

#[cfg(feature = "execute")]
fn object<'a>(value: &'a Value, name: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| anyhow!("{name} must be an object"))
}

#[cfg(feature = "execute")]
fn canonical(value: &Value, kind: Option<GeometryKind>) -> Result<Value> {
    use geo::Validation;
    let value = canonicalize_geometry(value, kind)?;
    to_geo(&value)?
        .check_validation()
        .map_err(|error| anyhow!("Invalid geometry: {error}"))?;
    Ok(value)
}

#[cfg(feature = "execute")]
fn point_coordinates(value: &Value) -> Result<(f64, f64)> {
    let point = canonical(value, Some(GeometryKind::Point))?;
    let coordinates = point["coordinates"]
        .as_array()
        .ok_or_else(|| anyhow!("Point coordinates must be an array"))?;
    Ok((
        coordinates[0].as_f64().unwrap(),
        coordinates[1].as_f64().unwrap(),
    ))
}

#[cfg(feature = "execute")]
fn feature_parts(value: &Value, name: &str) -> Result<(Value, Value)> {
    let feature = object(value, name)?;
    if feature.get("type").and_then(Value::as_str) != Some("Feature") {
        bail!("{name} must have GeoJSON type Feature");
    }
    let geometry = feature
        .get("geometry")
        .ok_or_else(|| anyhow!("{name} requires a geometry member"))?;
    if geometry.is_null() {
        bail!("{name} has a null geometry, which cannot become a Geometry value");
    }
    let properties = match feature.get("properties") {
        None | Some(Value::Null) => json!({}),
        Some(value) => {
            object(value, "Feature properties")?;
            value.clone()
        }
    };
    Ok((canonical(geometry, None)?, properties))
}

#[cfg(feature = "execute")]
fn polyline_precision(inputs: &Value) -> Result<(u32, f64)> {
    let precision = integer(inputs, "precision", 0, 10)? as u32;
    Ok((precision, 10_f64.powi(precision as i32)))
}

#[cfg(feature = "execute")]
fn encode_polyline_value(value: i64, output: &mut String) -> Result<()> {
    let mut encoded = if value < 0 {
        (value.unsigned_abs() << 1).saturating_sub(1)
    } else {
        (value as u64) << 1
    };
    while encoded >= 0x20 {
        let code = ((encoded & 0x1f) | 0x20) + 63;
        output.push(char::from_u32(code as u32).ok_or_else(|| anyhow!("Invalid polyline"))?);
        encoded >>= 5;
    }
    output.push(char::from_u32((encoded + 63) as u32).ok_or_else(|| anyhow!("Invalid polyline"))?);
    Ok(())
}

#[cfg(feature = "execute")]
fn decode_polyline_value(bytes: &[u8], cursor: &mut usize) -> Result<i64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *bytes
            .get(*cursor)
            .ok_or_else(|| anyhow!("Encoded polyline ended inside a coordinate"))?;
        *cursor += 1;
        if !(63..=126).contains(&byte) {
            bail!("Encoded polyline contains an invalid character");
        }
        let chunk = u64::from(byte - 63);
        if shift >= 64 || (chunk & 0x1f) > (u64::MAX >> shift) {
            bail!("Encoded polyline coordinate is too large");
        }
        result |= (chunk & 0x1f) << shift;
        if chunk < 0x20 {
            break;
        }
        shift += 5;
        if shift > 60 {
            bail!("Encoded polyline coordinate is too large");
        }
    }
    let magnitude = (result >> 1) as i64;
    Ok(if result & 1 == 1 {
        -magnitude - 1
    } else {
        magnitude
    })
}

#[cfg(feature = "execute")]
fn h3_resolution(inputs: &Value) -> Result<h3o::Resolution> {
    h3o::Resolution::try_from(integer(inputs, "resolution", 0, 15)? as u8)
        .map_err(|error| anyhow!("Invalid H3 resolution: {error}"))
}

#[cfg(feature = "execute")]
fn parse_cells(value: &Value) -> Result<Vec<h3o::CellIndex>> {
    use std::{collections::HashSet, str::FromStr};
    let values = value
        .as_array()
        .ok_or_else(|| anyhow!("cells must be an array"))?;
    if values.len() > 100_000 {
        bail!("cells cannot contain more than 100000 indexes");
    }
    let mut cells = Vec::with_capacity(values.len());
    let mut seen = HashSet::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let text = value
            .as_str()
            .ok_or_else(|| anyhow!("cells[{index}] must be a string"))?;
        let cell = h3o::CellIndex::from_str(text)
            .map_err(|error| anyhow!("Invalid H3 cell at index {index}: {error}"))?;
        if seen.insert(cell) {
            cells.push(cell);
        }
    }
    Ok(cells)
}

#[cfg(feature = "execute")]
fn execute(operation: Operation, inputs: &Value) -> Result<Vec<(&'static str, Value)>> {
    use Operation::*;
    match operation {
        FeatureGeometry => {
            let (geometry, properties) = feature_parts(input(inputs, "feature")?, "feature")?;
            Ok(vec![("geometry_out", geometry), ("properties", properties)])
        }
        MakeFeature => {
            let geometry = canonical(input(inputs, "geometry")?, None)?;
            let properties = input(inputs, "properties")?;
            object(properties, "properties")?;
            let id = input(inputs, "id")?
                .as_str()
                .ok_or_else(|| anyhow!("id must be a string"))?;
            let mut feature = json!({
                "type":"Feature",
                "geometry":geometry,
                "properties":properties
            });
            if !id.is_empty() {
                feature
                    .as_object_mut()
                    .unwrap()
                    .insert("id".into(), json!(id));
            }
            Ok(vec![("feature", feature)])
        }
        FeatureCollectionGeometries => {
            let collection = object(input(inputs, "feature_collection")?, "feature_collection")?;
            if collection.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
                bail!("feature_collection must have GeoJSON type FeatureCollection");
            }
            let features = collection
                .get("features")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("FeatureCollection requires a features array"))?;
            let mut geometries = Vec::with_capacity(features.len());
            let mut properties = Vec::with_capacity(features.len());
            for (index, feature) in features.iter().enumerate() {
                let (geometry, feature_properties) =
                    feature_parts(feature, &format!("features[{index}]"))?;
                geometries.push(geometry);
                properties.push(feature_properties);
            }
            let geometry = canonical(
                &json!({"type":"GeometryCollection", "geometries":geometries}),
                Some(GeometryKind::GeometryCollection),
            )?;
            Ok(vec![
                ("geometry_out", geometry),
                ("properties", Value::Array(properties)),
            ])
        }
        MakeFeatureCollection => {
            let geometries = input(inputs, "geometries")?
                .as_array()
                .ok_or_else(|| anyhow!("geometries must be an array"))?;
            let properties = input(inputs, "properties")?
                .as_array()
                .ok_or_else(|| anyhow!("properties must be an array"))?;
            if !properties.is_empty() && properties.len() != geometries.len() {
                bail!(
                    "properties must be empty or contain one object per Geometry: received {} properties for {} geometries",
                    properties.len(),
                    geometries.len()
                );
            }
            let features = geometries
                .iter()
                .enumerate()
                .map(|(index, geometry)| -> Result<Value> {
                    let geometry = canonical(geometry, None)?;
                    let properties = properties.get(index).cloned().unwrap_or_else(|| json!({}));
                    object(&properties, &format!("properties[{index}]"))?;
                    Ok(json!({
                        "type":"Feature",
                        "geometry":geometry,
                        "properties":properties
                    }))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(vec![(
                "feature_collection",
                json!({"type":"FeatureCollection", "features":features}),
            )])
        }
        EncodePolyline => {
            let geometry = canonical(input(inputs, "geometry")?, Some(GeometryKind::LineString))?;
            let (_, factor) = polyline_precision(inputs)?;
            let coordinates = geometry["coordinates"].as_array().unwrap();
            let mut previous_latitude = 0i64;
            let mut previous_longitude = 0i64;
            let mut polyline = String::with_capacity(coordinates.len() * 8);
            for position in coordinates {
                let position = position.as_array().unwrap();
                let longitude = (position[0].as_f64().unwrap() * factor).round() as i64;
                let latitude = (position[1].as_f64().unwrap() * factor).round() as i64;
                encode_polyline_value(latitude - previous_latitude, &mut polyline)?;
                encode_polyline_value(longitude - previous_longitude, &mut polyline)?;
                previous_latitude = latitude;
                previous_longitude = longitude;
            }
            Ok(vec![("polyline", json!(polyline))])
        }
        DecodePolyline => {
            let polyline = input(inputs, "polyline")?
                .as_str()
                .ok_or_else(|| anyhow!("polyline must be a string"))?;
            let (_, factor) = polyline_precision(inputs)?;
            let bytes = polyline.as_bytes();
            let mut cursor = 0usize;
            let mut latitude = 0i64;
            let mut longitude = 0i64;
            let mut coordinates = Vec::new();
            while cursor < bytes.len() {
                latitude = latitude
                    .checked_add(decode_polyline_value(bytes, &mut cursor)?)
                    .ok_or_else(|| anyhow!("Decoded latitude overflowed"))?;
                longitude = longitude
                    .checked_add(decode_polyline_value(bytes, &mut cursor)?)
                    .ok_or_else(|| anyhow!("Decoded longitude overflowed"))?;
                coordinates.push(json!([longitude as f64 / factor, latitude as f64 / factor]));
                if coordinates.len() > 100_000 {
                    bail!("Encoded polyline contains more than 100000 positions");
                }
            }
            let geometry = canonical(
                &json!({"type":"LineString", "coordinates":coordinates}),
                Some(GeometryKind::LineString),
            )?;
            Ok(vec![("geometry_out", geometry)])
        }
        PointToH3Cell => {
            let (longitude, latitude) = point_coordinates(input(inputs, "geometry")?)?;
            let coordinate = h3o::LatLng::new(latitude, longitude)
                .map_err(|error| anyhow!("Invalid Point for H3: {error}"))?;
            let cell = coordinate.to_cell(h3_resolution(inputs)?);
            Ok(vec![("cell", json!(cell.to_string()))])
        }
        H3CellBoundary => {
            use std::str::FromStr;
            let cell = input(inputs, "cell")?
                .as_str()
                .ok_or_else(|| anyhow!("cell must be a string"))?;
            let cell = h3o::CellIndex::from_str(cell)
                .map_err(|error| anyhow!("Invalid H3 cell: {error}"))?;
            let mut ring = cell
                .boundary()
                .iter()
                .map(|coordinate| json!([coordinate.lng(), coordinate.lat()]))
                .collect::<Vec<_>>();
            if let Some(first) = ring.first().cloned() {
                ring.push(first);
            }
            let geometry = canonical(
                &json!({"type":"Polygon", "coordinates":[ring]}),
                Some(GeometryKind::Polygon),
            )?;
            Ok(vec![("geometry_out", geometry)])
        }
        H3CellsToGeometry => {
            use h3o::geom::SolventBuilder;
            let cells = parse_cells(input(inputs, "cells")?)?;
            if let Some(resolution) = cells.first().map(|cell| cell.resolution())
                && cells.iter().any(|cell| cell.resolution() != resolution)
            {
                bail!("All H3 cells must use the same resolution");
            }
            let geometry = SolventBuilder::new()
                .build()
                .dissolve(cells)
                .map_err(|error| anyhow!("Failed to dissolve H3 cells: {error}"))?;
            let geometry = canonical(
                &from_geo(&geo::Geometry::MultiPolygon(geometry))?,
                Some(GeometryKind::MultiPolygon),
            )?;
            Ok(vec![("geometry_out", geometry)])
        }
        PolygonToH3Cells => {
            use geo::{Geometry, Validation};
            use h3o::geom::{ContainmentMode, TilerBuilder};
            let geometry = to_geo(&canonical(input(inputs, "geometry")?, None)?)?;
            geometry
                .check_validation()
                .map_err(|error| anyhow!("Invalid geometry for H3 coverage: {error}"))?;
            let containment = match input(inputs, "containment")?
                .as_str()
                .ok_or_else(|| anyhow!("containment must be a string"))?
            {
                "centroid" => ContainmentMode::ContainsCentroid,
                "contains-boundary" => ContainmentMode::ContainsBoundary,
                "intersects-boundary" => ContainmentMode::IntersectsBoundary,
                "covers" => ContainmentMode::Covers,
                value => bail!(
                    "Unknown containment mode {value}; expected centroid, contains-boundary, intersects-boundary or covers"
                ),
            };
            let mut tiler = TilerBuilder::new(h3_resolution(inputs)?)
                .containment_mode(containment)
                .build();
            match geometry {
                Geometry::Polygon(polygon) => tiler
                    .add(polygon)
                    .map_err(|error| anyhow!("Invalid Polygon for H3 coverage: {error}"))?,
                Geometry::MultiPolygon(polygons) => tiler
                    .add_batch(polygons.0)
                    .map_err(|error| anyhow!("Invalid MultiPolygon for H3 coverage: {error}"))?,
                _ => bail!("H3 coverage requires Polygon or MultiPolygon"),
            }
            let maximum = integer(inputs, "max_cells", 1, 1_000_000)? as usize;
            let mut cells = tiler
                .into_coverage()
                .take(maximum + 1)
                .map(|cell| cell.to_string())
                .collect::<Vec<_>>();
            if cells.len() > maximum {
                bail!("H3 coverage exceeds max_cells ({maximum})");
            }
            cells.sort_unstable();
            cells.dedup();
            Ok(vec![("cells", json!(cells))])
        }
        EncodeGeohash => {
            let (longitude, latitude) = point_coordinates(input(inputs, "geometry")?)?;
            let precision = integer(inputs, "precision", 1, 12)? as usize;
            let geohash = geohash::encode(
                geohash::Coord {
                    x: longitude,
                    y: latitude,
                },
                precision,
            )
            .map_err(|error| anyhow!("Failed to encode geohash: {error}"))?;
            Ok(vec![("geohash", json!(geohash))])
        }
        DecodeGeohash => {
            let hash = input(inputs, "geohash")?
                .as_str()
                .ok_or_else(|| anyhow!("geohash must be a string"))?;
            if !(1..=12).contains(&hash.len()) {
                bail!("geohash must contain 1 through 12 ASCII characters");
            }
            let (center, _, _) = geohash::decode(hash)
                .map_err(|error| anyhow!("Failed to decode geohash: {error}"))?;
            let bounds = geohash::decode_bbox(hash)
                .map_err(|error| anyhow!("Failed to decode geohash bounds: {error}"))?;
            let west = bounds.min().x;
            let south = bounds.min().y;
            let east = bounds.max().x;
            let north = bounds.max().y;
            let center = canonical(
                &json!({"type":"Point", "coordinates":[center.x,center.y]}),
                Some(GeometryKind::Point),
            )?;
            let bounds = canonical(
                &json!({
                    "type":"Polygon",
                    "coordinates":[[
                        [west,south], [east,south], [east,north], [west,north], [west,south]
                    ]]
                }),
                Some(GeometryKind::Polygon),
            )?;
            Ok(vec![("center", center), ("bounds", bounds)])
        }
    }
}

#[cfg(feature = "execute")]
async fn run_operation(operation: Operation, context: &mut ExecutionContext) -> Result<()> {
    let mut inputs = Map::new();
    for pin in definition(operation)
        .pins
        .values()
        .filter(|pin| pin.pin_type == PinType::Input)
    {
        let value = context.evaluate_pin::<Value>(&pin.name).await?;
        inputs.insert(pin.name.clone(), value);
    }
    for (name, value) in execute(operation, &Value::Object(inputs))? {
        context.set_pin_value(name, value).await?;
    }
    Ok(())
}

macro_rules! implement_node {
    ($name:ident, $operation:expr) => {
        #[async_trait]
        impl NodeLogic for $name {
            fn get_node(&self) -> Node {
                definition($operation)
            }

            #[cfg(feature = "execute")]
            async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
                run_operation($operation, context).await
            }

            #[cfg(not(feature = "execute"))]
            async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
                Err(flow_like_types::anyhow!(
                    "Geometry operations require the execute feature"
                ))
            }
        }
    };
}

#[crate::register_node]
#[derive(Default)]
pub struct GeometryFeatureGeometryNode;
implement_node!(GeometryFeatureGeometryNode, Operation::FeatureGeometry);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryMakeFeatureNode;
implement_node!(GeometryMakeFeatureNode, Operation::MakeFeature);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryFeatureCollectionGeometriesNode;
implement_node!(
    GeometryFeatureCollectionGeometriesNode,
    Operation::FeatureCollectionGeometries
);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryMakeFeatureCollectionNode;
implement_node!(
    GeometryMakeFeatureCollectionNode,
    Operation::MakeFeatureCollection
);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryEncodePolylineNode;
implement_node!(GeometryEncodePolylineNode, Operation::EncodePolyline);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryDecodePolylineNode;
implement_node!(GeometryDecodePolylineNode, Operation::DecodePolyline);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryPointToH3CellNode;
implement_node!(GeometryPointToH3CellNode, Operation::PointToH3Cell);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryH3CellBoundaryNode;
implement_node!(GeometryH3CellBoundaryNode, Operation::H3CellBoundary);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryH3CellsToGeometryNode;
implement_node!(GeometryH3CellsToGeometryNode, Operation::H3CellsToGeometry);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryPolygonToH3CellsNode;
implement_node!(GeometryPolygonToH3CellsNode, Operation::PolygonToH3Cells);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryEncodeGeohashNode;
implement_node!(GeometryEncodeGeohashNode, Operation::EncodeGeohash);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryDecodeGeohashNode;
implement_node!(GeometryDecodeGeohashNode, Operation::DecodeGeohash);

#[cfg(test)]
mod definition_tests {
    use super::*;

    fn pin(node: &Node, name: &str) -> Pin {
        node.pins
            .values()
            .find(|pin| pin.name == name)
            .unwrap()
            .clone()
    }

    #[test]
    fn geometry_integrations_keep_geometry_pins_typed() {
        let point_to_h3 = definition(Operation::PointToH3Cell);
        assert_eq!(
            pin(&point_to_h3, "geometry").schema.as_deref(),
            Some(marker(GeometryKind::Point))
        );
        let feature_collection = definition(Operation::FeatureCollectionGeometries);
        assert_eq!(
            pin(&feature_collection, "geometry_out").schema.as_deref(),
            Some(marker(GeometryKind::GeometryCollection))
        );
    }

    #[test]
    fn array_contracts_are_explicit() {
        assert_eq!(
            pin(&definition(Operation::PolygonToH3Cells), "cells").value_type,
            ValueType::Array
        );
        assert_eq!(
            pin(&definition(Operation::MakeFeatureCollection), "geometries").value_type,
            ValueType::Array
        );
    }
}

#[cfg(all(test, feature = "execute"))]
mod execution_tests {
    use super::*;

    fn output(operation: Operation, inputs: Value, name: &str) -> Value {
        execute(operation, &inputs)
            .unwrap()
            .into_iter()
            .find(|(pin, _)| *pin == name)
            .unwrap()
            .1
    }

    #[test]
    fn encoded_polyline_matches_reference_example() {
        let line = json!({
            "type":"LineString",
            "coordinates":[[-120.2,38.5],[-120.95,40.7],[-126.453,43.252]]
        });
        let encoded = output(
            Operation::EncodePolyline,
            json!({"geometry":line, "precision":5}),
            "polyline",
        );
        assert_eq!(encoded, json!("_p~iF~ps|U_ulLnnqC_mqNvxq`@"));
        let decoded = output(
            Operation::DecodePolyline,
            json!({"polyline":encoded, "precision":5}),
            "geometry_out",
        );
        assert_eq!(decoded["type"], "LineString");
        assert_eq!(decoded["coordinates"][2], json!([-126.453, 43.252]));
    }

    #[test]
    fn feature_round_trip_keeps_geometry_and_properties() {
        let feature = json!({
            "type":"Feature",
            "id":"place-1",
            "geometry":{"type":"Point", "coordinates":[13.4,52.5]},
            "properties":{"name":"Berlin"}
        });
        let values = execute(Operation::FeatureGeometry, &json!({"feature":feature})).unwrap();
        assert_eq!(values[0].1["coordinates"], json!([13.4, 52.5]));
        assert_eq!(values[1].1["name"], "Berlin");
    }

    #[test]
    fn geohash_decode_returns_geometry_values() {
        let values = execute(Operation::DecodeGeohash, &json!({"geohash":"u33dc1"})).unwrap();
        assert_eq!(values[0].1["type"], "Point");
        assert_eq!(values[1].1["type"], "Polygon");
    }
}
