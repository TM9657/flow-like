//! Planar overlay, topology, validity, and buffer nodes for native Geometry values.

use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::{Pin, PinOptions, PinType, ValueType},
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
use flow_like_types::{Result, Value, anyhow, bail, geometry::canonicalize_geometry};
#[cfg(feature = "execute")]
use geo::{
    BooleanOps, Buffer, Coord, CoordsIter, Distance, Euclidean, Geometry, GeometryCollection,
    LineString, MultiLineString, MultiPoint, MultiPolygon, Polygon, Relate, Validation,
    algorithm::{
        buffer::{BufferStyle, LineCap, LineJoin},
        unary_union,
    },
};

const DEFAULT_STYLE_ANGLE_DEGREES: f64 = 11.459_155_902_616_466;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Union,
    Difference,
    SymmetricDifference,
    UnaryUnion,
    ClipLine,
    Covers,
    CoveredBy,
    Touches,
    Crosses,
    Overlaps,
    TopologicallyEquals,
    Disjoint,
    DWithin,
    RelatePattern,
    IsTopologicallyValid,
    ValidityReason,
    ValidationErrors,
    IsEmpty,
    Repair,
    PlanarBuffer,
}

fn geometry_input<'a>(node: &'a mut Node, name: &str, kind: Option<GeometryKind>) -> &'a mut Pin {
    let pin = node.add_input_pin(
        name,
        name,
        "Validated two-dimensional WGS 84 GeoJSON geometry",
        VariableType::Geometry,
    );
    pin.schema = kind.map(|kind| marker(kind).to_string());
    pin
}

fn geometry_output(node: &mut Node, kind: Option<GeometryKind>) {
    let pin = node.add_output_pin(
        "geometry_out",
        "Geometry",
        "Validated two-dimensional WGS 84 GeoJSON geometry",
        VariableType::Geometry,
    );
    pin.schema = kind.map(|kind| marker(kind).to_string());
}

fn data_output<'a>(
    node: &'a mut Node,
    name: &str,
    ty: VariableType,
    description: &str,
) -> &'a mut Pin {
    node.add_output_pin(name, name, description, ty)
}

fn definition(operation: Operation) -> Node {
    use Operation::*;
    let (id, alias, title, description) = match operation {
        Union => (
            "geometry_union",
            "union",
            "Geometry Union (Planar)",
            "Combines Polygon or MultiPolygon regions in the longitude/latitude coordinate plane. Returns a MultiPolygon.",
        ),
        Difference => (
            "geometry_difference",
            "difference",
            "Geometry Difference (Planar)",
            "Subtracts Polygon or MultiPolygon B from A in the longitude/latitude coordinate plane. Returns a MultiPolygon.",
        ),
        SymmetricDifference => (
            "geometry_symmetric_difference",
            "symmetricDifference",
            "Geometry Symmetric Difference (Planar)",
            "Returns Polygon or MultiPolygon regions belonging to exactly one input. Returns a MultiPolygon.",
        ),
        UnaryUnion => (
            "geometry_unary_union",
            "unaryUnion",
            "Dissolve Geometries (Planar)",
            "Efficiently dissolves an array of Polygon and MultiPolygon values. An empty array returns an empty MultiPolygon.",
        ),
        ClipLine => (
            "geometry_clip_line",
            "clipLine",
            "Clip Line by Polygon (Planar)",
            "Returns the portions of a LineString or MultiLineString inside a Polygon or MultiPolygon mask, including its boundary.",
        ),
        Covers => (
            "geometry_covers",
            "covers",
            "Geometry Covers (Planar)",
            "Tests whether every point of B lies in the interior or boundary of A using DE-9IM semantics.",
        ),
        CoveredBy => (
            "geometry_covered_by",
            "coveredBy",
            "Geometry Covered By (Planar)",
            "Tests whether every point of A lies in the interior or boundary of B using DE-9IM semantics.",
        ),
        Touches => (
            "geometry_touches",
            "touches",
            "Geometry Touches (Planar)",
            "Tests whether the inputs share boundary points but have disjoint interiors using DE-9IM semantics.",
        ),
        Crosses => (
            "geometry_crosses",
            "crosses",
            "Geometry Crosses (Planar)",
            "Tests whether the inputs cross using DE-9IM semantics.",
        ),
        Overlaps => (
            "geometry_overlaps",
            "overlaps",
            "Geometry Overlaps (Planar)",
            "Tests whether same-dimensional inputs partly overlap without either covering the other.",
        ),
        TopologicallyEquals => (
            "geometry_topologically_equals",
            "topologicallyEquals",
            "Geometry Topologically Equals (Planar)",
            "Tests whether both inputs describe the same point set using DE-9IM semantics. Coordinate order may differ.",
        ),
        Disjoint => (
            "geometry_disjoint",
            "disjoint",
            "Geometry Disjoint (Planar)",
            "Tests whether the inputs share no point using DE-9IM semantics.",
        ),
        DWithin => (
            "geometry_d_within",
            "dWithin",
            "Geometry Within Distance (Degrees)",
            "Tests whether the shortest planar distance between two non-empty geometries is at most the supplied coordinate-degree distance.",
        ),
        RelatePattern => (
            "geometry_relate_pattern",
            "relatePattern",
            "Geometry Relate Pattern (Planar)",
            "Tests a DE-9IM relation pattern. The pattern has nine characters chosen from T, F, *, 0, 1, and 2.",
        ),
        IsTopologicallyValid => (
            "geometry_is_topologically_valid",
            "isTopologicallyValid",
            "Is Geometry Topologically Valid",
            "Checks OGC Simple Feature topology after the Geometry value has passed structural validation.",
        ),
        ValidityReason => (
            "geometry_validity_reason",
            "validityReason",
            "Geometry Validity Reason",
            "Returns the first OGC topology error, or an empty string when the geometry is topologically valid.",
        ),
        ValidationErrors => (
            "geometry_validation_errors",
            "validationErrors",
            "Geometry Validation Errors",
            "Returns all topology errors currently detectable by the validator. A valid geometry returns an empty array.",
        ),
        IsEmpty => (
            "geometry_is_empty",
            "isEmpty",
            "Is Geometry Empty",
            "Tests whether a multi-geometry or GeometryCollection contains no coordinate positions.",
        ),
        Repair => (
            "geometry_repair",
            "repair",
            "Repair Geometry",
            "Repairs a limited set of safe topology defects: consecutive duplicate coordinates, ring closure, overlapping polygon parts, and polygon self-intersections that planar overlay can resolve. Collections are repaired recursively. Other defects return an error.",
        ),
        PlanarBuffer => (
            "geometry_planar_buffer",
            "planarBuffer",
            "Geometry Buffer (Degrees)",
            "Buffers a geometry in coordinate degrees. The operation is planar, does not wrap at the antimeridian, and rejects results outside WGS 84 coordinate bounds.",
        ),
    };

    let mut node = Node::new(id, title, description, "Web/Geo/Geometry");
    node.set_flowscript_name("geometry", alias);
    node.add_icon("/flow/icons/map.svg");

    match operation {
        Union | Difference | SymmetricDifference => {
            geometry_input(&mut node, "a", None);
            geometry_input(&mut node, "b", None);
            geometry_output(&mut node, Some(GeometryKind::MultiPolygon));
        }
        UnaryUnion => {
            geometry_input(&mut node, "geometries", None)
                .set_value_type(ValueType::Array)
                .set_default_value(Some(json!([])));
            geometry_output(&mut node, Some(GeometryKind::MultiPolygon));
        }
        ClipLine => {
            geometry_input(&mut node, "line", None);
            geometry_input(&mut node, "mask", None);
            geometry_output(&mut node, Some(GeometryKind::MultiLineString));
        }
        Covers | CoveredBy | Touches | Crosses | Overlaps | TopologicallyEquals | Disjoint => {
            geometry_input(&mut node, "a", None);
            geometry_input(&mut node, "b", None);
            data_output(
                &mut node,
                "result",
                VariableType::Boolean,
                "Planar predicate result",
            );
        }
        DWithin => {
            geometry_input(&mut node, "a", None);
            geometry_input(&mut node, "b", None);
            node.add_input_pin(
                "distance",
                "distance",
                "Maximum planar distance in coordinate degrees",
                VariableType::Float,
            )
            .set_default_value(Some(json!(0.0)))
            .set_options(PinOptions::new().set_range((0.0, 403.0)).build());
            data_output(
                &mut node,
                "result",
                VariableType::Boolean,
                "Whether the geometries are within the requested distance",
            );
        }
        RelatePattern => {
            geometry_input(&mut node, "a", None);
            geometry_input(&mut node, "b", None);
            node.add_input_pin(
                "pattern",
                "pattern",
                "Nine-character DE-9IM pattern using T, F, *, 0, 1, and 2",
                VariableType::String,
            )
            .set_default_value(Some(json!("*********")));
            data_output(
                &mut node,
                "result",
                VariableType::Boolean,
                "Whether the relation matches the pattern",
            );
        }
        IsTopologicallyValid => {
            geometry_input(&mut node, "geometry", None);
            data_output(
                &mut node,
                "result",
                VariableType::Boolean,
                "Whether the geometry satisfies OGC topology rules",
            );
        }
        ValidityReason => {
            geometry_input(&mut node, "geometry", None);
            data_output(
                &mut node,
                "reason",
                VariableType::String,
                "First topology error, or an empty string",
            );
        }
        ValidationErrors => {
            geometry_input(&mut node, "geometry", None);
            data_output(
                &mut node,
                "errors",
                VariableType::String,
                "Topology validation errors",
            )
            .set_value_type(ValueType::Array);
        }
        IsEmpty => {
            geometry_input(&mut node, "geometry", None);
            data_output(
                &mut node,
                "result",
                VariableType::Boolean,
                "Whether the geometry has no coordinate positions",
            );
        }
        Repair => {
            geometry_input(&mut node, "geometry", None);
            geometry_output(&mut node, None);
        }
        PlanarBuffer => {
            geometry_input(&mut node, "geometry", None);
            node.add_input_pin(
                "distance",
                "distance",
                "Signed buffer distance in coordinate degrees. Negative distances shrink polygonal inputs.",
                VariableType::Float,
            )
            .set_default_value(Some(json!(0.0)))
            .set_options(PinOptions::new().set_range((-360.0, 360.0)).build());
            node.add_input_pin(
                "cap",
                "cap",
                "Line endpoint style: Round, Square, or Butt",
                VariableType::String,
            )
            .set_default_value(Some(json!("Round")))
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["Round".into(), "Square".into(), "Butt".into()])
                    .build(),
            );
            node.add_input_pin(
                "join",
                "join",
                "Corner style: Round, Miter, or Bevel",
                VariableType::String,
            )
            .set_default_value(Some(json!("Round")))
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["Round".into(), "Miter".into(), "Bevel".into()])
                    .build(),
            );
            node.add_input_pin(
                "arc_step_degrees",
                "arc_step_degrees",
                "Maximum angular step used to approximate round caps and joins",
                VariableType::Float,
            )
            .set_default_value(Some(json!(DEFAULT_STYLE_ANGLE_DEGREES)))
            .set_options(PinOptions::new().set_range((1.8, 45.0)).build());
            node.add_input_pin(
                "miter_min_angle_degrees",
                "miter_min_angle_degrees",
                "Minimum corner angle that retains a miter instead of beveling it",
                VariableType::Float,
            )
            .set_default_value(Some(json!(DEFAULT_STYLE_ANGLE_DEGREES)))
            .set_options(PinOptions::new().set_range((1.8, 178.2)).build());
            geometry_output(&mut node, Some(GeometryKind::MultiPolygon));
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
macro_rules! ensure {
    ($condition:expr, $($message:tt)*) => {
        if !$condition { bail!($($message)*); }
    };
}

#[cfg(feature = "execute")]
fn input<'a>(inputs: &'a Value, name: &str) -> Result<&'a Value> {
    inputs
        .get(name)
        .ok_or_else(|| anyhow!("Missing geometry input {name}"))
}

#[cfg(feature = "execute")]
fn number(inputs: &Value, name: &str) -> Result<f64> {
    let value = input(inputs, name)?
        .as_f64()
        .ok_or_else(|| anyhow!("{name} must be a finite number"))?;
    ensure!(value.is_finite(), "{name} must be finite");
    Ok(value)
}

#[cfg(feature = "execute")]
fn text<'a>(inputs: &'a Value, name: &str) -> Result<&'a str> {
    input(inputs, name)?
        .as_str()
        .ok_or_else(|| anyhow!("{name} must be a string"))
}

#[cfg(feature = "execute")]
fn raw_geometry(value: &Value) -> Result<Geometry<f64>> {
    to_geo(&canonicalize_geometry(value, None)?)
}

#[cfg(feature = "execute")]
fn checked_geometry(value: &Value) -> Result<Geometry<f64>> {
    let geometry = raw_geometry(value)?;
    geometry
        .check_validation()
        .map_err(|error| anyhow!("Invalid geometry for spatial operation: {error}"))?;
    Ok(geometry)
}

#[cfg(feature = "execute")]
fn polygon_set(geometry: Geometry<f64>) -> Result<MultiPolygon<f64>> {
    match geometry {
        Geometry::Polygon(polygon) => Ok(MultiPolygon(vec![polygon])),
        Geometry::MultiPolygon(polygons) => Ok(polygons),
        _ => bail!("This operation requires Polygon or MultiPolygon"),
    }
}

#[cfg(feature = "execute")]
fn line_set(geometry: Geometry<f64>) -> Result<MultiLineString<f64>> {
    match geometry {
        Geometry::LineString(line) => Ok(MultiLineString(vec![line])),
        Geometry::MultiLineString(lines) => Ok(lines),
        _ => bail!("This operation requires LineString or MultiLineString"),
    }
}

#[cfg(feature = "execute")]
fn encoded_geometry(geometry: Geometry<f64>, kind: GeometryKind) -> Result<Value> {
    geometry
        .check_validation()
        .map_err(|error| anyhow!("Spatial operation produced invalid geometry: {error}"))?;
    Ok(canonicalize_geometry(&from_geo(&geometry)?, Some(kind))?)
}

#[cfg(feature = "execute")]
fn geometry_result(
    geometry: Geometry<f64>,
    kind: GeometryKind,
) -> Result<Vec<(&'static str, Value)>> {
    Ok(vec![("geometry_out", encoded_geometry(geometry, kind)?)])
}

#[cfg(feature = "execute")]
fn any_geometry_result(geometry: Geometry<f64>) -> Result<Vec<(&'static str, Value)>> {
    geometry
        .check_validation()
        .map_err(|error| anyhow!("Geometry repair produced invalid geometry: {error}"))?;
    Ok(vec![(
        "geometry_out",
        canonicalize_geometry(&from_geo(&geometry)?, None)?,
    )])
}

#[cfg(feature = "execute")]
fn same_coord(a: Coord<f64>, b: Coord<f64>) -> bool {
    a.x == b.x && a.y == b.y
}

/// Remove only adjacent duplicate positions. Removing non-adjacent positions can change topology.
#[cfg(feature = "execute")]
fn clean_line(line: LineString<f64>, ring: bool) -> Result<LineString<f64>> {
    let mut coordinates: Vec<Coord<f64>> = Vec::with_capacity(line.0.len() + usize::from(ring));
    for coordinate in line.0 {
        if coordinates
            .last()
            .is_none_or(|previous| !same_coord(*previous, coordinate))
        {
            coordinates.push(coordinate);
        }
    }

    if ring {
        let first = coordinates
            .first()
            .copied()
            .ok_or_else(|| anyhow!("Cannot repair an empty polygon ring"))?;
        if coordinates
            .last()
            .is_none_or(|last| !same_coord(*last, first))
        {
            coordinates.push(first);
        }
        let mut distinct = Vec::new();
        for coordinate in coordinates.iter().copied().take(coordinates.len() - 1) {
            if !distinct
                .iter()
                .any(|existing| same_coord(*existing, coordinate))
            {
                distinct.push(coordinate);
            }
        }
        ensure!(
            distinct.len() >= 3,
            "Cannot safely repair a polygon ring with fewer than three distinct vertices"
        );
    } else {
        ensure!(
            coordinates.len() >= 2,
            "Cannot safely repair a LineString with fewer than two distinct positions"
        );
    }
    Ok(LineString::new(coordinates))
}

#[cfg(feature = "execute")]
fn clean_polygon(polygon: Polygon<f64>) -> Result<Polygon<f64>> {
    let exterior = clean_line(polygon.exterior().clone(), true)?;
    let interiors = polygon
        .interiors()
        .iter()
        .cloned()
        .map(|ring| clean_line(ring, true))
        .collect::<Result<Vec<_>>>()?;
    Ok(Polygon::new(exterior, interiors))
}

#[cfg(feature = "execute")]
fn overlay_repair(polygons: MultiPolygon<f64>) -> Result<MultiPolygon<f64>> {
    if polygons.is_valid() {
        return Ok(polygons);
    }
    let original_error = polygons
        .check_validation()
        .expect_err("invalidity checked above")
        .to_string();

    let overlaid = unary_union(polygons.0.iter());
    if !overlaid.0.is_empty() && overlaid.is_valid() {
        return Ok(overlaid);
    }
    let buffered = polygons.buffer(0.0);
    if !buffered.0.is_empty() && buffered.is_valid() {
        return Ok(buffered);
    }
    bail!("Geometry cannot be safely repaired: {original_error}")
}

#[cfg(feature = "execute")]
fn repair_polygon(polygon: Polygon<f64>) -> Result<Geometry<f64>> {
    let mut repaired = overlay_repair(MultiPolygon(vec![clean_polygon(polygon)?]))?;
    if repaired.0.len() == 1 {
        Ok(Geometry::Polygon(repaired.0.remove(0)))
    } else {
        Ok(Geometry::MultiPolygon(repaired))
    }
}

#[cfg(feature = "execute")]
fn repair_geometry(geometry: Geometry<f64>) -> Result<Geometry<f64>> {
    let repaired = match geometry {
        Geometry::Point(point) => Geometry::Point(point),
        Geometry::MultiPoint(points) => {
            let mut unique = Vec::with_capacity(points.0.len());
            for point in points.0 {
                if !unique
                    .iter()
                    .any(|existing: &geo::Point<f64>| same_coord(existing.0, point.0))
                {
                    unique.push(point);
                }
            }
            Geometry::MultiPoint(MultiPoint(unique))
        }
        Geometry::LineString(line) => Geometry::LineString(clean_line(line, false)?),
        Geometry::MultiLineString(lines) => Geometry::MultiLineString(MultiLineString(
            lines
                .0
                .into_iter()
                .map(|line| clean_line(line, false))
                .collect::<Result<Vec<_>>>()?,
        )),
        Geometry::Polygon(polygon) => return repair_polygon(polygon),
        Geometry::MultiPolygon(polygons) => {
            let cleaned = polygons
                .0
                .into_iter()
                .map(clean_polygon)
                .collect::<Result<Vec<_>>>()?;
            Geometry::MultiPolygon(overlay_repair(MultiPolygon(cleaned))?)
        }
        Geometry::GeometryCollection(collection) => {
            Geometry::GeometryCollection(GeometryCollection(
                collection
                    .0
                    .into_iter()
                    .map(repair_geometry)
                    .collect::<Result<Vec<_>>>()?,
            ))
        }
        _ => bail!("Unsupported geometry variant for repair"),
    };
    repaired
        .check_validation()
        .map_err(|error| anyhow!("Geometry cannot be safely repaired: {error}"))?;
    Ok(repaired)
}

#[cfg(feature = "execute")]
fn validation_errors(inputs: &Value) -> Result<Vec<String>> {
    Ok(raw_geometry(input(inputs, "geometry")?)?
        .validation_errors()
        .into_iter()
        .map(|error| error.to_string())
        .collect())
}

#[cfg(feature = "execute")]
fn execute(operation: Operation, inputs: &Value) -> Result<Vec<(&'static str, Value)>> {
    use Operation::*;
    match operation {
        Union | Difference | SymmetricDifference => {
            let a = polygon_set(checked_geometry(input(inputs, "a")?)?)?;
            let b = polygon_set(checked_geometry(input(inputs, "b")?)?)?;
            let result = match operation {
                Union => a.union(&b),
                Difference => a.difference(&b),
                SymmetricDifference => a.xor(&b),
                _ => unreachable!(),
            };
            geometry_result(result.into(), GeometryKind::MultiPolygon)
        }
        UnaryUnion => {
            let values = input(inputs, "geometries")?
                .as_array()
                .ok_or_else(|| anyhow!("geometries must be an array"))?;
            let mut polygons = Vec::new();
            for value in values {
                polygons.extend(polygon_set(checked_geometry(value)?)?.0);
            }
            let result = if polygons.is_empty() {
                MultiPolygon(vec![])
            } else {
                unary_union(polygons.iter())
            };
            geometry_result(result.into(), GeometryKind::MultiPolygon)
        }
        ClipLine => {
            let line = line_set(checked_geometry(input(inputs, "line")?)?)?;
            let mask = polygon_set(checked_geometry(input(inputs, "mask")?)?)?;
            geometry_result(
                mask.clip(&line, false).into(),
                GeometryKind::MultiLineString,
            )
        }
        Covers | CoveredBy | Touches | Crosses | Overlaps | TopologicallyEquals | Disjoint => {
            let a = checked_geometry(input(inputs, "a")?)?;
            let b = checked_geometry(input(inputs, "b")?)?;
            let relation = a.relate(&b);
            let result = match operation {
                Covers => relation.is_covers(),
                CoveredBy => relation.is_coveredby(),
                Touches => relation.is_touches(),
                Crosses => relation.is_crosses(),
                Overlaps => relation.is_overlaps(),
                TopologicallyEquals => relation.is_equal_topo(),
                Disjoint => relation.is_disjoint(),
                _ => unreachable!(),
            };
            Ok(vec![("result", json!(result))])
        }
        DWithin => {
            let a = checked_geometry(input(inputs, "a")?)?;
            let b = checked_geometry(input(inputs, "b")?)?;
            let distance = number(inputs, "distance")?;
            ensure!(
                (0.0..=403.0).contains(&distance),
                "distance must be between 0 and 403 coordinate degrees"
            );
            ensure!(
                a.coords_iter().next().is_some() && b.coords_iter().next().is_some(),
                "Empty geometries have no distance"
            );
            Ok(vec![(
                "result",
                json!(Euclidean.distance(&a, &b) <= distance),
            )])
        }
        RelatePattern => {
            let a = checked_geometry(input(inputs, "a")?)?;
            let b = checked_geometry(input(inputs, "b")?)?;
            let pattern = text(inputs, "pattern")?;
            let result = a
                .relate(&b)
                .matches(pattern)
                .map_err(|error| anyhow!("Invalid DE-9IM pattern: {error}"))?;
            Ok(vec![("result", json!(result))])
        }
        IsTopologicallyValid => Ok(vec![(
            "result",
            json!(raw_geometry(input(inputs, "geometry")?)?.is_valid()),
        )]),
        ValidityReason => {
            let reason = validation_errors(inputs)?
                .into_iter()
                .next()
                .unwrap_or_default();
            Ok(vec![("reason", json!(reason))])
        }
        ValidationErrors => Ok(vec![("errors", json!(validation_errors(inputs)?))]),
        IsEmpty => Ok(vec![(
            "result",
            json!(
                raw_geometry(input(inputs, "geometry")?)?
                    .coords_iter()
                    .next()
                    .is_none()
            ),
        )]),
        Repair => any_geometry_result(repair_geometry(raw_geometry(input(inputs, "geometry")?)?)?),
        PlanarBuffer => {
            let geometry = checked_geometry(input(inputs, "geometry")?)?;
            let distance = number(inputs, "distance")?;
            ensure!(
                (-360.0..=360.0).contains(&distance),
                "distance must be between -360 and 360 degrees"
            );
            let arc_step = number(inputs, "arc_step_degrees")?;
            ensure!(
                (1.8..=45.0).contains(&arc_step),
                "arc_step_degrees must be between 1.8 and 45"
            );
            let miter_angle = number(inputs, "miter_min_angle_degrees")?;
            ensure!(
                (1.8..=178.2).contains(&miter_angle),
                "miter_min_angle_degrees must be between 1.8 and 178.2"
            );
            let arc_step = arc_step.to_radians();
            let mut style = BufferStyle::new(distance);
            style = style.line_cap(match text(inputs, "cap")? {
                "Round" => LineCap::Round(arc_step),
                "Square" => LineCap::Square,
                "Butt" => LineCap::Butt,
                value => bail!("Unknown buffer cap style: {value}"),
            });
            style = style.line_join(match text(inputs, "join")? {
                "Round" => LineJoin::Round(arc_step),
                "Miter" => LineJoin::Miter(miter_angle.to_radians()),
                "Bevel" => LineJoin::Bevel,
                value => bail!("Unknown buffer join style: {value}"),
            });
            geometry_result(
                geometry.buffer_with_style(style).into(),
                GeometryKind::MultiPolygon,
            )
        }
    }
}

#[cfg(feature = "execute")]
async fn run_operation(operation: Operation, context: &mut ExecutionContext) -> Result<()> {
    let mut values = flow_like_types::json::Map::new();
    for pin in definition(operation)
        .pins
        .values()
        .filter(|pin| pin.pin_type == PinType::Input)
    {
        values.insert(
            pin.name.clone(),
            context.evaluate_pin::<Value>(&pin.name).await?,
        );
    }
    for (name, value) in execute(operation, &Value::Object(values))? {
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
pub struct GeometryUnionNode;
implement_node!(GeometryUnionNode, Operation::Union);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryDifferenceNode;
implement_node!(GeometryDifferenceNode, Operation::Difference);

#[crate::register_node]
#[derive(Default)]
pub struct GeometrySymmetricDifferenceNode;
implement_node!(
    GeometrySymmetricDifferenceNode,
    Operation::SymmetricDifference
);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryUnaryUnionNode;
implement_node!(GeometryUnaryUnionNode, Operation::UnaryUnion);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryClipLineNode;
implement_node!(GeometryClipLineNode, Operation::ClipLine);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryCoversNode;
implement_node!(GeometryCoversNode, Operation::Covers);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryCoveredByNode;
implement_node!(GeometryCoveredByNode, Operation::CoveredBy);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryTouchesNode;
implement_node!(GeometryTouchesNode, Operation::Touches);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryCrossesNode;
implement_node!(GeometryCrossesNode, Operation::Crosses);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryOverlapsNode;
implement_node!(GeometryOverlapsNode, Operation::Overlaps);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryTopologicallyEqualsNode;
implement_node!(
    GeometryTopologicallyEqualsNode,
    Operation::TopologicallyEquals
);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryDisjointNode;
implement_node!(GeometryDisjointNode, Operation::Disjoint);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryDWithinNode;
implement_node!(GeometryDWithinNode, Operation::DWithin);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryRelatePatternNode;
implement_node!(GeometryRelatePatternNode, Operation::RelatePattern);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryIsTopologicallyValidNode;
implement_node!(
    GeometryIsTopologicallyValidNode,
    Operation::IsTopologicallyValid
);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryValidityReasonNode;
implement_node!(GeometryValidityReasonNode, Operation::ValidityReason);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryValidationErrorsNode;
implement_node!(GeometryValidationErrorsNode, Operation::ValidationErrors);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryIsEmptyNode;
implement_node!(GeometryIsEmptyNode, Operation::IsEmpty);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryRepairNode;
implement_node!(GeometryRepairNode, Operation::Repair);

#[crate::register_node]
#[derive(Default)]
pub struct GeometryPlanarBufferNode;
implement_node!(GeometryPlanarBufferNode, Operation::PlanarBuffer);

#[cfg(all(test, feature = "execute"))]
mod tests {
    use super::*;
    use geo::Area;

    fn output(operation: Operation, inputs: Value, name: &str) -> Value {
        execute(operation, &inputs)
            .unwrap()
            .into_iter()
            .find(|(output_name, _)| *output_name == name)
            .unwrap()
            .1
    }

    fn square(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Value {
        json!({
            "type": "Polygon",
            "coordinates": [[
                [min_x, min_y],
                [max_x, min_y],
                [max_x, max_y],
                [min_x, max_y],
                [min_x, min_y]
            ]]
        })
    }

    fn point(x: f64, y: f64) -> Value {
        json!({"type": "Point", "coordinates": [x, y]})
    }

    fn area(value: &Value) -> f64 {
        to_geo(value).unwrap().unsigned_area()
    }

    #[test]
    fn polygon_boolean_operations_have_set_semantics() {
        let inputs = json!({"a": square(0.0, 0.0, 2.0, 2.0), "b": square(1.0, 1.0, 3.0, 3.0)});
        assert_eq!(
            area(&output(Operation::Union, inputs.clone(), "geometry_out")),
            7.0
        );
        assert_eq!(
            area(&output(
                Operation::Difference,
                inputs.clone(),
                "geometry_out"
            )),
            3.0
        );
        assert_eq!(
            area(&output(
                Operation::SymmetricDifference,
                inputs,
                "geometry_out"
            )),
            6.0
        );
    }

    #[test]
    fn unary_union_accepts_polygons_multipolygons_and_empty_arrays() {
        let dissolved = output(
            Operation::UnaryUnion,
            json!({"geometries": [square(0.0, 0.0, 1.0, 1.0), square(1.0, 0.0, 2.0, 1.0)]}),
            "geometry_out",
        );
        assert_eq!(area(&dissolved), 2.0);
        assert_eq!(dissolved["coordinates"].as_array().unwrap().len(), 1);

        let empty = output(
            Operation::UnaryUnion,
            json!({"geometries": []}),
            "geometry_out",
        );
        assert_eq!(empty, json!({"type": "MultiPolygon", "coordinates": []}));
    }

    #[test]
    fn clip_line_returns_only_segments_inside_mask() {
        let clipped = output(
            Operation::ClipLine,
            json!({
                "line": {"type": "LineString", "coordinates": [[-1.0, 0.5], [2.0, 0.5]]},
                "mask": square(0.0, 0.0, 1.0, 1.0)
            }),
            "geometry_out",
        );
        assert_eq!(clipped["type"], "MultiLineString");
        assert_eq!(clipped["coordinates"], json!([[[0.0, 0.5], [1.0, 0.5]]]));
    }

    #[test]
    fn de9im_predicates_cover_boundaries_and_set_relations() {
        let region = square(0.0, 0.0, 2.0, 2.0);
        let boundary_point = point(0.0, 1.0);
        assert_eq!(
            output(
                Operation::Covers,
                json!({"a": region, "b": boundary_point}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::CoveredBy,
                json!({"a": boundary_point, "b": region}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::Touches,
                json!({"a": square(0.0, 0.0, 1.0, 1.0), "b": square(1.0, 0.0, 2.0, 1.0)}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::Overlaps,
                json!({"a": square(0.0, 0.0, 2.0, 2.0), "b": square(1.0, 1.0, 3.0, 3.0)}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::Disjoint,
                json!({"a": point(0.0, 0.0), "b": point(1.0, 1.0)}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::TopologicallyEquals,
                json!({
                    "a": {"type": "LineString", "coordinates": [[0.0, 0.0], [1.0, 1.0]]},
                    "b": {"type": "LineString", "coordinates": [[1.0, 1.0], [0.0, 0.0]]}
                }),
                "result"
            ),
            json!(true)
        );
    }

    #[test]
    fn crosses_d_within_and_raw_relate_pattern_are_available() {
        let horizontal = json!({"type": "LineString", "coordinates": [[0.0, 1.0], [2.0, 1.0]]});
        let vertical = json!({"type": "LineString", "coordinates": [[1.0, 0.0], [1.0, 2.0]]});
        assert_eq!(
            output(
                Operation::Crosses,
                json!({"a": horizontal, "b": vertical}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::DWithin,
                json!({"a": point(0.0, 0.0), "b": point(0.3, 0.4), "distance": 0.5}),
                "result"
            ),
            json!(true)
        );
        assert_eq!(
            output(
                Operation::RelatePattern,
                json!({"a": point(1.0, 1.0), "b": point(1.0, 1.0), "pattern": "T********"}),
                "result"
            ),
            json!(true)
        );
        assert!(
            execute(
                Operation::RelatePattern,
                &json!({"a": point(1.0, 1.0), "b": point(1.0, 1.0), "pattern": "bad"})
            )
            .is_err()
        );
    }

    #[test]
    fn validity_nodes_report_invalid_topology_without_rejecting_the_value() {
        let bow_tie = json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0], [0.0, 0.0]]]
        });
        assert_eq!(
            output(
                Operation::IsTopologicallyValid,
                json!({"geometry": bow_tie}),
                "result"
            ),
            json!(false)
        );
        assert!(
            output(
                Operation::ValidityReason,
                json!({"geometry": bow_tie}),
                "reason"
            )
            .as_str()
            .is_some_and(|reason| !reason.is_empty())
        );
        assert!(
            !output(
                Operation::ValidationErrors,
                json!({"geometry": bow_tie}),
                "errors"
            )
            .as_array()
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn repair_removes_redundant_positions_recursively_and_rejects_collapsed_lines() {
        let repeated = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0]
            ]]
        });
        let repaired = output(
            Operation::Repair,
            json!({
                "geometry": {
                    "type": "GeometryCollection",
                    "geometries": [repeated]
                }
            }),
            "geometry_out",
        );
        assert!(to_geo(&repaired).unwrap().is_valid());
        assert_eq!(
            repaired["geometries"][0]["coordinates"][0]
                .as_array()
                .unwrap()
                .len(),
            5
        );

        assert!(
            execute(
                Operation::Repair,
                &json!({
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[0.0, 0.0], [0.0, 0.0]]
                    }
                })
            )
            .is_err()
        );
    }

    #[test]
    fn empty_and_buffer_nodes_handle_edges_and_validate_style() {
        assert_eq!(
            output(
                Operation::IsEmpty,
                json!({"geometry": {"type": "GeometryCollection", "geometries": []}}),
                "result"
            ),
            json!(true)
        );
        let buffer_inputs = json!({
            "geometry": point(10.0, 10.0),
            "distance": 0.1,
            "cap": "Round",
            "join": "Round",
            "arc_step_degrees": DEFAULT_STYLE_ANGLE_DEGREES,
            "miter_min_angle_degrees": DEFAULT_STYLE_ANGLE_DEGREES
        });
        let buffered = output(Operation::PlanarBuffer, buffer_inputs, "geometry_out");
        assert!((area(&buffered) - std::f64::consts::PI * 0.01).abs() < 0.001);

        assert!(
            execute(
                Operation::PlanarBuffer,
                &json!({
                    "geometry": point(10.0, 10.0),
                    "distance": 0.1,
                    "cap": "Round",
                    "join": "Round",
                    "arc_step_degrees": 0.0,
                    "miter_min_angle_degrees": DEFAULT_STYLE_ANGLE_DEGREES
                })
            )
            .is_err()
        );
    }

    #[test]
    fn node_definitions_expose_bounded_buffer_controls_and_subtyped_outputs() {
        let buffer = definition(Operation::PlanarBuffer);
        let arc_step = buffer
            .pins
            .values()
            .find(|pin| pin.name == "arc_step_degrees")
            .unwrap();
        assert_eq!(arc_step.options.as_ref().unwrap().range, Some((1.8, 45.0)));
        let output = buffer
            .pins
            .values()
            .find(|pin| pin.name == "geometry_out")
            .unwrap();
        assert_eq!(
            output.schema.as_deref(),
            Some(marker(GeometryKind::MultiPolygon))
        );
    }
}
