//! Geometry fields, GeoArrow conversion, and application SQL registration.

use arrow_array::{Array, ArrayRef, BinaryArray, RecordBatch};
use arrow_schema::{DataType, Field};
use flow_like_types::{Result, Value, anyhow};
use geoarrow_array::GeoArrowArray;
use std::{collections::HashMap, sync::Arc};

pub const EXTENSION_NAME: &str = "ARROW:extension:name";
pub const EXTENSION_METADATA: &str = "ARROW:extension:metadata";
pub const WGS84_METADATA: &str = r#"{"crs":"EPSG:4326","crs_type":"authority_code"}"#;

pub fn is_geometry_field(field: &Field) -> bool {
    field
        .metadata()
        .get(EXTENSION_NAME)
        .is_some_and(|name| name.starts_with("geoarrow."))
}

pub fn contains_geometry_field(field: &Field) -> bool {
    fn children(data_type: &DataType) -> bool {
        match data_type {
            DataType::Struct(fields) => fields.iter().any(|field| contains_geometry_field(field)),
            DataType::List(field)
            | DataType::LargeList(field)
            | DataType::FixedSizeList(field, _)
            | DataType::ListView(field)
            | DataType::LargeListView(field)
            | DataType::Map(field, _)
            | DataType::RunEndEncoded(_, field) => contains_geometry_field(field),
            DataType::Union(fields, _) => fields
                .iter()
                .any(|(_, field)| contains_geometry_field(field)),
            DataType::Dictionary(_, value) => children(value),
            _ => false,
        }
    }
    is_geometry_field(field) || children(field.data_type())
}

/// V1 table writes support scalar geometry columns. Nested geometry reads remain available.
pub fn validate_geometry_field(field: &Field) -> Result<()> {
    if !is_geometry_field(field) && contains_geometry_field(field) {
        return Err(anyhow!(
            "Column '{}' contains nested geometry; declare a scalar geometry column for validated writes",
            field.name()
        ));
    }
    if is_geometry_field(field) {
        validate_crs(field)?;
    }
    Ok(())
}

pub fn geometry_field(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::Binary, nullable).with_metadata(HashMap::from([
        (EXTENSION_NAME.into(), "geoarrow.wkb".into()),
        (EXTENSION_METADATA.into(), WGS84_METADATA.into()),
    ]))
}

/// Preserves semantic column types alongside JSON rows.
pub fn property_metadata(
    schema: &arrow_schema::Schema,
) -> HashMap<String, HashMap<String, String>> {
    schema
        .fields()
        .iter()
        .filter(|field| !field.metadata().is_empty())
        .map(|field| (field.name().clone(), field.metadata().clone()))
        .collect()
}

pub fn validate_crs(field: &Field) -> Result<()> {
    let metadata: Value = serde_json::from_str(
        field
            .metadata()
            .get(EXTENSION_METADATA)
            .map(String::as_str)
            .unwrap_or("{}"),
    )?;
    let crs = metadata.get("crs");
    let known = crs.and_then(Value::as_str).is_some_and(|crs| {
        matches!(
            crs,
            "EPSG:4326" | "OGC:CRS84" | "urn:ogc:def:crs:OGC::CRS84"
        )
    }) || crs.is_some_and(|crs| {
        let id = &crs["id"];
        id["authority"].as_str() == Some("EPSG")
            && (id["code"].as_u64() == Some(4326) || id["code"].as_str() == Some("4326"))
    });
    if !known {
        return Err(anyhow!(
            "Geometry column '{}' has unknown or unsupported CRS. Use flow_geomfromtext(WKT) to explicitly import WGS84 longitude/latitude coordinates",
            field.name()
        ));
    }
    if metadata
        .get("edges")
        .is_some_and(|edges| !edges.is_null() && edges.as_str() != Some("planar"))
    {
        return Err(anyhow!(
            "Geometry column '{}' uses unsupported edge semantics",
            field.name()
        ));
    }
    Ok(())
}

/// Decode every supported GeoArrow layout through its WKB representation.
pub fn decode_column(array: &dyn Array, field: &Field) -> Result<Vec<Value>> {
    validate_crs(field)?;
    let geo = geoarrow_array::array::from_arrow_array(array, field)?;
    let wkb = geoarrow_array::cast::to_wkb::<i32>(geo.as_ref())?.to_array_ref();
    let binary = wkb
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| anyhow!("GeoArrow WKB conversion did not return Binary"))?;
    binary
        .iter()
        .map(|bytes| {
            bytes
                .map(flow_like_geometry::from_wkb)
                .transpose()
                .map(|value| value.unwrap_or(Value::Null))
        })
        .collect()
}

pub fn decode_value(array: &dyn Array, field: &Field, row: usize) -> Result<Value> {
    if row >= array.len() {
        return Err(anyhow!("Geometry row index out of bounds"));
    }
    Ok(decode_column(array.slice(row, 1).as_ref(), field)?.remove(0))
}

/// Validate geometry columns supplied as Arrow, before any database mutation.
pub fn validate_batch(batch: &RecordBatch) -> Result<()> {
    for (field, array) in batch.schema().fields().iter().zip(batch.columns()) {
        validate_geometry_field(field)?;
        if is_geometry_field(field) {
            decode_column(array.as_ref(), field)?;
        }
    }
    Ok(())
}

/// Convert direct Arrow geometry input to the declared WKB schema.
pub fn normalize_batch(
    batch: &RecordBatch,
    target: &arrow_schema::SchemaRef,
) -> Result<RecordBatch> {
    if batch.num_columns() != target.fields().len() {
        return Err(anyhow!(
            "Arrow insert column count differs from the declared table schema"
        ));
    }
    let mut arrays = Vec::with_capacity(target.fields().len());
    for field in target.fields() {
        validate_geometry_field(field)?;
        let index = batch.schema().index_of(field.name())?;
        let source = batch.schema().field(index).clone();
        let array = batch.column(index);
        if is_geometry_field(field) {
            if !is_geometry_field(&source) {
                return Err(anyhow!(
                    "Column '{}' requires declared GeoArrow input with WGS84 metadata",
                    field.name()
                ));
            }
            validate_crs(field)?;
            let values = decode_column(array.as_ref(), &source)?;
            if field.data_type() != &DataType::Binary {
                if source.data_type() != field.data_type()
                    || source.metadata().get(EXTENSION_NAME) != field.metadata().get(EXTENSION_NAME)
                {
                    return Err(anyhow!(
                        "Native GeoArrow targets require matching geometry kinds and Arrow layouts"
                    ));
                }
                arrays.push(array.clone());
                continue;
            }
            let bytes = values
                .iter()
                .map(|value| {
                    if value.is_null() {
                        Ok(None)
                    } else {
                        flow_like_geometry::to_wkb(value).map(Some)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            arrays.push(Arc::new(BinaryArray::from_iter(
                bytes.iter().map(|bytes| bytes.as_deref()),
            )) as ArrayRef);
        } else {
            if is_geometry_field(&source) {
                return Err(anyhow!(
                    "Geometry column '{}' requires an explicitly declared geometry target",
                    field.name()
                ));
            }
            arrays.push(array.clone());
        }
    }
    let result = RecordBatch::try_new(target.clone(), arrays)?;
    validate_batch(&result)?;
    Ok(result)
}

/// Register spatial SQL and an explicit WGS84 import operation.
pub fn register_geo_functions(context: &datafusion::prelude::SessionContext) {
    geodatafusion::register(context);
    context.register_udf(datafusion::logical_expr::ScalarUDF::from(
        Wgs84FromText::default(),
    ));
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Wgs84FromText {
    signature: datafusion::logical_expr::Signature,
}
impl Default for Wgs84FromText {
    fn default() -> Self {
        Self {
            signature: datafusion::logical_expr::Signature::exact(
                vec![DataType::Utf8],
                datafusion::logical_expr::Volatility::Immutable,
            ),
        }
    }
}
impl datafusion::logical_expr::ScalarUDFImpl for Wgs84FromText {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn name(&self) -> &str {
        "flow_geomfromtext"
    }
    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.signature
    }
    fn return_type(&self, _: &[DataType]) -> datafusion::error::Result<DataType> {
        Ok(DataType::Binary)
    }
    fn return_field_from_args(
        &self,
        _: datafusion::logical_expr::ReturnFieldArgs,
    ) -> datafusion::error::Result<Arc<Field>> {
        Ok(Arc::new(geometry_field("flow_geomfromtext", true)))
    }
    fn invoke_with_args(
        &self,
        args: datafusion::logical_expr::ScalarFunctionArgs,
    ) -> datafusion::error::Result<datafusion::logical_expr::ColumnarValue> {
        use datafusion::{
            common::ScalarValue, error::DataFusionError, logical_expr::ColumnarValue,
        };
        let arrays = ColumnarValue::values_to_arrays(&args.args)?;
        let input = arrays[0]
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .ok_or_else(|| {
                DataFusionError::Execution("flow_geomfromtext requires WKT text".into())
            })?;
        let values = input
            .iter()
            .map(|text| {
                text.map(|text| {
                    flow_like_geometry::from_wkt(text)
                        .and_then(|value| flow_like_geometry::to_wkb(&value))
                })
                .transpose()
            })
            .collect::<Result<Vec<_>>>()
            .map_err(|e| DataFusionError::Execution(e.to_string()))?;
        if matches!(args.args[0], ColumnarValue::Scalar(_)) {
            Ok(ColumnarValue::Scalar(ScalarValue::Binary(
                values.into_iter().next().flatten(),
            )))
        } else {
            Ok(ColumnarValue::Array(Arc::new(BinaryArray::from_iter(
                values.iter().map(|value| value.as_deref()),
            ))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn binary_is_not_implicitly_geometry() {
        assert!(!is_geometry_field(&Field::new(
            "bytes",
            DataType::Binary,
            true
        )));
        assert!(validate_crs(&Field::new("unknown", DataType::Binary, true)).is_err());
    }
    #[test]
    fn geometry_metadata_accepts_explicit_planar_edges() {
        let field = |edges: &str| {
            geometry_field("geom", true).with_metadata(HashMap::from([
                (EXTENSION_NAME.into(), "geoarrow.wkb".into()),
                (
                    EXTENSION_METADATA.into(),
                    format!(r#"{{"crs":"EPSG:4326","edges":"{edges}"}}"#),
                ),
            ]))
        };
        assert!(validate_crs(&field("planar")).is_ok());
        assert!(validate_crs(&field("spherical")).is_err());
    }

    #[test]
    fn geometry_nested_writes_are_rejected_and_null_parents_remain_null() -> Result<()> {
        let child = Arc::new(geometry_field("geom", true));
        let children = arrow_schema::Fields::from(vec![child.clone()]);
        let nested = arrow_array::StructArray::new(
            children.clone(),
            vec![Arc::new(BinaryArray::from(vec![Some(&[1_u8, 2][..])]))],
            Some(arrow::buffer::NullBuffer::from(vec![false])),
        );
        let batch = RecordBatch::try_new(
            Arc::new(arrow_schema::Schema::new(vec![Field::new(
                "nested",
                DataType::Struct(children),
                true,
            )])),
            vec![Arc::new(nested)],
        )?;
        assert!(validate_batch(&batch).is_err());
        assert_eq!(
            crate::arrow_utils::record_batch_to_value(&batch)?,
            vec![json!({"nested":null})]
        );
        let fields = batch.schema().fields().to_vec();
        assert!(
            crate::arrow_utils::value_to_record_batch_with_fields(
                vec![json!({"nested":null})],
                Some(fields)
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn geometry_native_writes_cannot_relabel_the_same_physical_layout() -> Result<()> {
        use arrow_array::{Float64Array, ListArray, StructArray};
        let xy = arrow_schema::Fields::from(vec![
            Field::new("x", DataType::Float64, false),
            Field::new("y", DataType::Float64, false),
        ]);
        let points = StructArray::new(
            xy.clone(),
            vec![
                Arc::new(Float64Array::from(vec![1., 3.])),
                Arc::new(Float64Array::from(vec![2., 4.])),
            ],
            None,
        );
        let vertices = Arc::new(Field::new("vertices", DataType::Struct(xy), false));
        let line = ListArray::new(
            vertices.clone(),
            arrow::buffer::OffsetBuffer::new(vec![0, 2].into()),
            Arc::new(points),
            None,
        );
        let field = |kind: &str| {
            Field::new("geom", DataType::List(vertices.clone()), true).with_metadata(HashMap::from(
                [
                    (EXTENSION_NAME.into(), kind.into()),
                    (EXTENSION_METADATA.into(), WGS84_METADATA.into()),
                ],
            ))
        };
        let batch = RecordBatch::try_new(
            Arc::new(arrow_schema::Schema::new(vec![field(
                "geoarrow.linestring",
            )])),
            vec![Arc::new(line)],
        )?;
        validate_batch(&batch)?;
        let target = Arc::new(arrow_schema::Schema::new(vec![field(
            "geoarrow.multipoint",
        )]));
        let error = normalize_batch(&batch, &target).unwrap_err().to_string();
        assert!(error.contains("matching geometry kinds"), "{error}");
        Ok(())
    }

    #[tokio::test]
    async fn geometry_arrow_inserts_use_authoritative_metadata() -> Result<()> {
        let ctx = datafusion::prelude::SessionContext::new();
        register_geo_functions(&ctx);
        let batches = ctx
            .sql("SELECT ST_Centroid(flow_geomfromtext('POINT(13 52)')) AS geom")
            .await?
            .collect()
            .await?;
        let target = Arc::new(arrow_schema::Schema::new(vec![geometry_field(
            "geom", true,
        )]));
        let normalized = normalize_batch(&batches[0], &target)?;
        assert_eq!(normalized.schema(), target);
        assert_eq!(
            decode_column(normalized.column(0).as_ref(), normalized.schema().field(0))?[0],
            json!({"type":"Point","coordinates":[13.,52.]})
        );
        let untagged = RecordBatch::try_new(
            Arc::new(arrow_schema::Schema::new(vec![Field::new(
                "geom",
                DataType::Binary,
                true,
            )])),
            normalized.columns().to_vec(),
        )?;
        assert!(normalize_batch(&untagged, &target).is_err());
        let unknown = ctx
            .sql("SELECT ST_GeomFromText('POINT(13 52)') AS geom")
            .await?
            .collect()
            .await?;
        assert!(normalize_batch(&unknown[0], &target).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn native_centroid_and_wkb_aliases_preserve_values_and_crs() -> Result<()> {
        let ctx = datafusion::prelude::SessionContext::new();
        register_geo_functions(&ctx);
        let batches = ctx.sql("SELECT ST_Centroid(flow_geomfromtext('LINESTRING(10 20, 20 30)')) AS center, ST_AsBinary(flow_geomfromtext('POINT(13.405 52.52)')) AS position").await?.collect().await?;
        assert_eq!(
            decode_column(batches[0].column(0).as_ref(), batches[0].schema().field(0))?[0],
            json!({"type":"Point","coordinates":[15.,25.]})
        );
        assert_eq!(
            decode_column(batches[0].column(1).as_ref(), batches[0].schema().field(1))?[0],
            json!({"type":"Point","coordinates":[13.405,52.52]})
        );
        let unknown = ctx
            .sql("SELECT ST_GeomFromText('POINT(1 2)')")
            .await?
            .collect()
            .await?;
        assert!(
            decode_column(unknown[0].column(0).as_ref(), unknown[0].schema().field(0)).is_err()
        );
        Ok(())
    }
}
