//! WGS84 geometry codecs shared by catalog nodes and storage.
//!
//! Importing WKT or WKB asserts longitude/latitude coordinates. Codecs validate
//! the shared two-dimensional profile and discard optional GeoJSON metadata.

use anyhow::{Result, bail, ensure};
use flow_like_types_contracts::geometry::canonicalize_geometry;
use geo_types::{
    Coord, Geometry, GeometryCollection, LineString, MultiLineString, MultiPoint, MultiPolygon,
    Point, Polygon,
};
use serde_json::{Value, json};
use wkt::ToWkt;

pub use geo_types;
const MAX_BYTES: usize = 1024 * 1024;
const MAX_DEPTH: usize = 32;
const MAX_POSITIONS: usize = 100_000;

/// Convert a validated GeoJSON object to an algorithm geometry.
pub fn to_geo(value: &Value) -> Result<Geometry<f64>> {
    let value = canonicalize_geometry(value, None)?;
    Ok(to_geo_validated(&value))
}

fn position(value: &Value) -> Coord<f64> {
    Coord {
        x: value[0].as_f64().unwrap(),
        y: value[1].as_f64().unwrap(),
    }
}
fn line(value: &Value) -> LineString<f64> {
    LineString(value.as_array().unwrap().iter().map(position).collect())
}
fn polygon(value: &Value) -> Polygon<f64> {
    let rings = value.as_array().unwrap();
    Polygon::new(line(&rings[0]), rings[1..].iter().map(line).collect())
}
fn to_geo_validated(value: &Value) -> Geometry<f64> {
    let coords = &value["coordinates"];
    match value["type"].as_str().unwrap() {
        "Point" => Point(position(coords)).into(),
        "LineString" => line(coords).into(),
        "Polygon" => polygon(coords).into(),
        "MultiPoint" => MultiPoint(
            coords
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Point(position(p)))
                .collect(),
        )
        .into(),
        "MultiLineString" => {
            MultiLineString(coords.as_array().unwrap().iter().map(line).collect()).into()
        }
        "MultiPolygon" => {
            MultiPolygon(coords.as_array().unwrap().iter().map(polygon).collect()).into()
        }
        "GeometryCollection" => Geometry::GeometryCollection(GeometryCollection(
            value["geometries"]
                .as_array()
                .unwrap()
                .iter()
                .map(to_geo_validated)
                .collect(),
        )),
        _ => unreachable!("validated geometry type"),
    }
}

fn point_json(coord: Coord<f64>) -> Result<Value> {
    ensure!(
        coord.x.is_finite() && coord.y.is_finite(),
        "Geometry operation produced a non-finite position"
    );
    Ok(json!([coord.x, coord.y]))
}
fn line_json(line: &LineString<f64>) -> Result<Value> {
    Ok(Value::Array(
        line.0
            .iter()
            .copied()
            .map(point_json)
            .collect::<Result<_>>()?,
    ))
}
fn polygon_json(polygon: &Polygon<f64>) -> Result<Value> {
    Ok(Value::Array(
        std::iter::once(polygon.exterior())
            .chain(polygon.interiors())
            .map(line_json)
            .collect::<Result<_>>()?,
    ))
}

/// Convert algorithm output to GeoJSON, rejecting non-finite or unsupported results.
pub fn from_geo(geometry: &Geometry<f64>) -> Result<Value> {
    let value = match geometry {
        Geometry::Point(p) => json!({"type":"Point", "coordinates":point_json(p.0)?}),
        Geometry::LineString(l) => json!({"type":"LineString", "coordinates":line_json(l)?}),
        Geometry::Polygon(p) => json!({"type":"Polygon", "coordinates":polygon_json(p)?}),
        Geometry::MultiPoint(p) => {
            json!({"type":"MultiPoint", "coordinates":p.0.iter().map(|p| point_json(p.0)).collect::<Result<Vec<_>>>()?})
        }
        Geometry::MultiLineString(l) => {
            json!({"type":"MultiLineString", "coordinates":l.0.iter().map(line_json).collect::<Result<Vec<_>>>()?})
        }
        Geometry::MultiPolygon(p) => {
            json!({"type":"MultiPolygon", "coordinates":p.0.iter().map(polygon_json).collect::<Result<Vec<_>>>()?})
        }
        Geometry::GeometryCollection(g) => {
            json!({"type":"GeometryCollection", "geometries":g.0.iter().map(from_geo).collect::<Result<Vec<_>>>()?})
        }
        Geometry::Rect(r) => return from_geo(&Geometry::Polygon(r.to_polygon())),
        Geometry::Triangle(t) => return from_geo(&Geometry::Polygon(t.to_polygon())),
        Geometry::Line(l) => {
            return from_geo(&Geometry::LineString(LineString(vec![l.start, l.end])));
        }
    };
    Ok(canonicalize_geometry(&value, None)?)
}

/// Parse two-dimensional WKT, asserting that its coordinates are WGS84 lon/lat.
pub fn from_wkt(text: &str) -> Result<Value> {
    ensure!(text.len() <= MAX_BYTES, "Geometry exceeds 1 MiB");
    let text = text.trim();
    let mut depth = 0usize;
    let mut closed_root = false;
    let mut has_parens = false;
    for ch in text.chars() {
        ensure!(
            !closed_root || ch.is_whitespace(),
            "Trailing data after WKT geometry"
        );
        if ch == '(' {
            has_parens = true;
            depth += 1;
            ensure!(depth <= MAX_DEPTH, "Geometry nesting exceeds 32");
        } else if ch == ')' {
            ensure!(depth > 0, "Unexpected closing parenthesis in WKT");
            depth -= 1;
            closed_root = depth == 0;
        }
    }
    ensure!(depth == 0, "Unclosed WKT geometry");
    if !has_parens {
        let words: Vec<_> = text.split_whitespace().collect();
        ensure!(
            words.len() == 2 && words[1].eq_ignore_ascii_case("EMPTY"),
            "Expected a two-dimensional WKT geometry or TYPE EMPTY"
        );
    }
    let parsed: wkt::Wkt<f64> = text
        .parse()
        .map_err(|err| anyhow::anyhow!("Invalid WKT: {err}"))?;
    fn check_polygon(polygon: &wkt::types::Polygon<f64>) -> Result<()> {
        ensure!(!polygon.rings().is_empty(), "Empty Polygon is unsupported");
        for ring in polygon.rings() {
            let coords = ring.coords();
            ensure!(
                coords.len() >= 4,
                "A polygon ring requires at least four positions"
            );
            let (first, last) = (coords.first().unwrap(), coords.last().unwrap());
            ensure!(
                first.x == last.x && first.y == last.y,
                "Polygon rings must be closed"
            );
        }
        Ok(())
    }
    fn check_dim(wkt: &wkt::Wkt<f64>) -> Result<()> {
        ensure!(
            wkt.dimension() == wkt::types::Dimension::XY,
            "Geometry supports only two-dimensional WKT"
        );
        if let wkt::Wkt::Point(point) = wkt {
            ensure!(
                point.coord().is_some(),
                "Empty Point is unsupported; use null for an absent value"
            );
        }
        if let wkt::Wkt::MultiPoint(points) = wkt {
            ensure!(
                points.points().iter().all(|point| point.coord().is_some()),
                "MultiPoint cannot contain empty points"
            );
        }
        if let wkt::Wkt::Polygon(polygon) = wkt {
            check_polygon(polygon)?;
        }
        if let wkt::Wkt::MultiPolygon(polygons) = wkt {
            for polygon in polygons.polygons() {
                check_polygon(polygon)?;
            }
        }
        if let wkt::Wkt::GeometryCollection(collection) = wkt {
            for child in collection.geometries() {
                check_dim(child)?;
            }
        }
        Ok(())
    }
    check_dim(&parsed)?;
    let geometry: Geometry<f64> = parsed
        .try_into()
        .map_err(|err| anyhow::anyhow!("Unsupported WKT geometry: {err}"))?;
    from_geo(&geometry)
}

/// Encode a geometry as two-dimensional WKT.
pub fn to_wkt(value: &Value) -> Result<String> {
    Ok(to_geo(value)?.to_wkt().to_string())
}

/// Encode ISO WKB in little-endian order. CRS lives in the containing field.
pub fn to_wkb(value: &Value) -> Result<Vec<u8>> {
    let value = canonicalize_geometry(value, None)?;
    let mut bytes = Vec::new();
    fn count(out: &mut Vec<u8>, n: usize) {
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    fn coords(out: &mut Vec<u8>, value: &Value, nesting: usize) {
        let values = value.as_array().unwrap();
        if nesting == 0 {
            for coordinate in values {
                out.extend_from_slice(&coordinate.as_f64().unwrap().to_le_bytes());
            }
        } else {
            count(out, values.len());
            for value in values {
                coords(out, value, nesting - 1);
            }
        }
    }
    fn geometry(out: &mut Vec<u8>, value: &Value) {
        let kind = match value["type"].as_str().unwrap() {
            "Point" => 1,
            "LineString" => 2,
            "Polygon" => 3,
            "MultiPoint" => 4,
            "MultiLineString" => 5,
            "MultiPolygon" => 6,
            "GeometryCollection" => 7,
            _ => unreachable!(),
        };
        out.push(1);
        count(out, kind);
        match kind {
            1..=3 => coords(out, &value["coordinates"], kind - 1),
            4..=6 => {
                let children = value["coordinates"].as_array().unwrap();
                count(out, children.len());
                let name = ["Point", "LineString", "Polygon"][kind - 4];
                for child in children {
                    geometry(out, &json!({"type":name,"coordinates":child}));
                }
            }
            7 => {
                let children = value["geometries"].as_array().unwrap();
                count(out, children.len());
                for child in children {
                    geometry(out, child);
                }
            }
            _ => unreachable!(),
        }
    }
    geometry(&mut bytes, &value);
    ensure!(bytes.len() <= MAX_BYTES, "Geometry WKB exceeds 1 MiB");
    Ok(bytes)
}

/// Decode bounded ISO WKB or 2D EWKB with SRID 4326. Reject trailing bytes.
pub fn from_wkb(bytes: &[u8]) -> Result<Value> {
    ensure!(bytes.len() <= MAX_BYTES, "Geometry WKB exceeds 1 MiB");
    struct Reader<'a> {
        bytes: &'a [u8],
        offset: usize,
        positions: usize,
    }
    impl Reader<'_> {
        fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
            let end = self
                .offset
                .checked_add(N)
                .ok_or_else(|| anyhow::anyhow!("WKB length overflow"))?;
            let value = self
                .bytes
                .get(self.offset..end)
                .ok_or_else(|| anyhow::anyhow!("Truncated WKB"))?;
            self.offset = end;
            Ok(value.try_into().unwrap())
        }
        fn uint(&mut self, le: bool) -> Result<u32> {
            let b = self.take()?;
            Ok(if le {
                u32::from_le_bytes(b)
            } else {
                u32::from_be_bytes(b)
            })
        }
        fn count(&mut self, le: bool) -> Result<usize> {
            let count = self.uint(le)? as usize;
            ensure!(
                count <= MAX_POSITIONS && count <= self.bytes.len().saturating_sub(self.offset),
                "Invalid or excessive WKB element count"
            );
            Ok(count)
        }
        fn position(&mut self, le: bool) -> Result<Value> {
            self.positions += 1;
            ensure!(
                self.positions <= MAX_POSITIONS,
                "Geometry has more than 100000 positions"
            );
            let x = self.take()?;
            let y = self.take()?;
            point_json(Coord {
                x: if le {
                    f64::from_le_bytes(x)
                } else {
                    f64::from_be_bytes(x)
                },
                y: if le {
                    f64::from_le_bytes(y)
                } else {
                    f64::from_be_bytes(y)
                },
            })
        }
        fn line(&mut self, le: bool) -> Result<Value> {
            let count = self.count(le)?;
            Ok(Value::Array(
                (0..count)
                    .map(|_| self.position(le))
                    .collect::<Result<_>>()?,
            ))
        }
        fn geometry(&mut self, depth: usize) -> Result<Value> {
            ensure!(depth <= MAX_DEPTH, "Geometry nesting exceeds 32");
            let le = match self.take::<1>()?[0] {
                0 => false,
                1 => true,
                _ => bail!("Invalid WKB byte order"),
            };
            let code = self.uint(le)?;
            ensure!(code & 0xc0000000 == 0, "Geometry supports only 2D WKB");
            if code & 0x20000000 != 0 {
                ensure!(self.uint(le)? == 4326, "EWKB SRID must be 4326");
            }
            let kind = code & !0x20000000;
            let names = [
                "Point",
                "LineString",
                "Polygon",
                "MultiPoint",
                "MultiLineString",
                "MultiPolygon",
                "GeometryCollection",
            ];
            ensure!(
                (1..=7).contains(&kind),
                "Unsupported WKB geometry type {kind}"
            );
            let value = match kind {
                1 => self.position(le)?,
                2 => self.line(le)?,
                3 => {
                    let count = self.count(le)?;
                    Value::Array((0..count).map(|_| self.line(le)).collect::<Result<_>>()?)
                }
                _ => {
                    let count = self.count(le)?;
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        let child = self.geometry(depth + 1)?;
                        if kind < 7 {
                            ensure!(
                                child["type"] == names[(kind - 4) as usize],
                                "WKB multi-geometry contains the wrong subtype"
                            );
                            values.push(child["coordinates"].clone());
                        } else {
                            values.push(child);
                        }
                    }
                    Value::Array(values)
                }
            };
            Ok(if kind == 7 {
                json!({"type":names[6],"geometries":value})
            } else {
                json!({"type":names[(kind-1) as usize],"coordinates":value})
            })
        }
    }
    let mut reader = Reader {
        bytes,
        offset: 0,
        positions: 0,
    };
    let value = reader.geometry(0)?;
    ensure!(
        reader.offset == bytes.len(),
        "Trailing bytes after WKB geometry"
    );
    Ok(canonicalize_geometry(&value, None)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_limit_is_checked_before_values_can_be_written() {
        let value = json!({"type":"LineString", "coordinates":vec![json!([0, 0]); 70_000]});
        assert!(serde_json::to_vec(&value).unwrap().len() < MAX_BYTES);
        assert!(to_wkb(&value).unwrap_err().to_string().contains("1 MiB"));
    }

    #[test]
    fn wkt_rejects_trailing_tokens_and_unbalanced_groups() {
        for text in [
            "POLYGON((0 0,1 0,1 1,0 1))",
            "MULTIPOLYGON(((0 0,1 0,1 1,0 1)))",
            "POINT(1 2) garbage",
            "POINT(1 2) POINT(3 4)",
            "MULTIPOINT EMPTY EMPTY",
            "POINT(1 2))",
            "POINT(1 2",
        ] {
            assert!(from_wkt(text).is_err(), "{text}");
        }
        assert!(from_wkt(" MULTIPOINT EMPTY ").is_ok());
    }

    #[test]
    fn roundtrips_all_types_and_empty_multi_geometries() -> Result<()> {
        let polygon = json!([[[0., 0.], [2., 0.], [2., 2.], [0., 0.]]]);
        let cases = vec![
            json!({"type":"Point","coordinates":[13.405,52.52]}),
            json!({"type":"LineString","coordinates":[[1.,2.],[3.,4.]]}),
            json!({"type":"Polygon","coordinates":polygon}),
            json!({"type":"MultiPoint","coordinates":[[1.,2.],[3.,4.]]}),
            json!({"type":"MultiLineString","coordinates":[[[1.,2.],[3.,4.]]]}),
            json!({"type":"MultiPolygon","coordinates":[polygon]}),
            json!({"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1.,2.]}]}),
            json!({"type":"MultiPoint","coordinates":[]}),
            json!({"type":"MultiLineString","coordinates":[]}),
            json!({"type":"MultiPolygon","coordinates":[]}),
            json!({"type":"GeometryCollection","geometries":[]}),
        ];
        for value in cases {
            assert_eq!(from_wkb(&to_wkb(&value)?)?, value);
            assert_eq!(from_wkt(&to_wkt(&value)?)?, value);
        }
        Ok(())
    }
    #[test]
    fn rejects_malformed_dimensions_crs_and_trailing_bytes() -> Result<()> {
        assert!(from_wkt("POINT Z (1 2 3)").is_err());
        assert!(from_wkt("GEOMETRYCOLLECTION(POINT Z (1 2 3))").is_err());
        assert!(from_wkt("POINT EMPTY").is_err());
        assert!(from_wkt("POINT (52 100)").is_err());
        let mut bytes = to_wkb(&json!({"type":"Point","coordinates":[1.,2.]}))?;
        bytes.push(0);
        assert!(from_wkb(&bytes).is_err());
        assert!(from_wkb(&[1, 7, 0, 0, 0, 255, 255, 255, 255]).is_err());
        assert!(from_wkb(&[1, 1]).is_err());
        Ok(())
    }
    #[test]
    fn accepts_big_endian_point() -> Result<()> {
        let mut bytes = vec![0, 0, 0, 0, 1];
        bytes.extend(13.405_f64.to_be_bytes());
        bytes.extend(52.52_f64.to_be_bytes());
        assert_eq!(
            from_wkb(&bytes)?,
            json!({"type":"Point","coordinates":[13.405,52.52]})
        );
        Ok(())
    }
}
