use flow_like_catalog_core::NodeDBConnection;
use flow_like_ml_core::{
    Annotation, InspectionSpec, LabelProvenance, Outcome, Sample, TaskKind, TensorData,
    content_digest,
};
use flow_like_ml_native::{
    dataset::{DatasetSplit, split_grouped, split_temporal},
    preprocessing::{FittedTabularPreprocessor, TabularOptions},
};
use flow_like_storage_contracts::database::DatabaseReference;
use flow_like_types::{Result, Value, anyhow, json::json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PinnedTableSource {
    pub database: NodeDBConnection,
    pub reference: DatabaseReference,
    #[serde(default)]
    pub locator: Option<TableLocator>,
    /// Temporal columns of the pinned schema whose values are not epoch milliseconds.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub temporal_units: BTreeMap<String, TemporalUnit>,
}

/// Storage unit of an Arrow timestamp or date column; sample timestamps convert it to milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TemporalUnit {
    Seconds,
    Microseconds,
    Nanoseconds,
    Days,
}
impl TemporalUnit {
    fn to_milliseconds(self, value: i64) -> Option<i64> {
        match self {
            Self::Seconds => value.checked_mul(1_000),
            Self::Microseconds => Some(value.div_euclid(1_000)),
            Self::Nanoseconds => Some(value.div_euclid(1_000_000)),
            Self::Days => value.checked_mul(86_400_000),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TableLocator {
    pub app_id: String,
    pub board_id: String,
    pub connection_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TableBudget {
    pub maximum_rows: usize,
    pub maximum_bytes: usize,
    pub maximum_tensor_elements: usize,
}
impl Default for TableBudget {
    fn default() -> Self {
        Self {
            maximum_rows: 100_000,
            maximum_bytes: 256 * 1024 * 1024,
            maximum_tensor_elements: 32 * 1024 * 1024,
        }
    }
}
impl TableBudget {
    pub fn validate(&self) -> Result<()> {
        if !(1..=1_000_000).contains(&self.maximum_rows)
            || !(1024..=1024 * 1024 * 1024).contains(&self.maximum_bytes)
            || !(1..=64 * 1024 * 1024).contains(&self.maximum_tensor_elements)
        {
            return Err(anyhow!("Invalid table row, byte or tensor budget"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FeatureSource {
    Tabular { options: TabularOptions },
    Tensor { column: String },
    Sample { column: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetSource {
    Column { column: String },
    Columns { columns: Vec<String> },
    Annotation { column: String },
    Sample,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TableMapping {
    pub row_id: String,
    pub group_id: Option<String>,
    pub timestamp_ms: Option<String>,
    pub window_start_ms: Option<String>,
    pub window_end_ms: Option<String>,
    pub outcome: Option<String>,
    pub features: FeatureSource,
    pub target: TargetSource,
    pub provenance: LabelProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TableSplitPolicy {
    Group {
        train_fraction: f64,
        validation_fraction: f64,
        seed: u64,
    },
    Time {
        train_end_ms: i64,
        validation_end_ms: i64,
        embargo_ms: u64,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TableDatasetRequest {
    pub source: NodeDBConnection,
    pub stream_id: String,
    pub spec: InspectionSpec,
    pub mapping: TableMapping,
    pub split: TableSplitPolicy,
    #[serde(default)]
    pub budget: TableBudget,
    #[serde(default)]
    pub selection: DatasetSelection,
    #[serde(default)]
    pub label_overrides: BTreeMap<String, TableLabelOverride>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DatasetSelection {
    pub excluded_ids: Vec<String>,
    pub excluded_groups: Vec<String>,
    pub ineligible_test_ids: Vec<String>,
    pub ineligible_test_groups: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TableLabelOverride {
    pub annotation: Annotation,
    pub provenance: LabelProvenance,
    pub annotation_revision: i64,
    #[serde(default)]
    pub outcome: Option<Outcome>,
    #[serde(default)]
    pub available_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ColumnProfile {
    pub name: String,
    pub missing: usize,
    pub numeric: usize,
    pub boolean: usize,
    pub text: usize,
    pub other: usize,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub examples: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TableProfile {
    pub partition: String,
    pub rows: usize,
    pub columns: Vec<ColumnProfile>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TablePreprocessing {
    Engineered {
        pipeline: flow_like_ml_native::feature_engineering::FittedFeaturePipeline,
        preprocessing: Box<TablePreprocessing>,
        #[serde(default)]
        projection: Option<flow_like_ml_native::reduction::FittedPca>,
        #[serde(default)]
        plan: Option<Box<super::feature_engineering::FeaturePlan>>,
    },
    Tabular {
        fitted: FittedTabularPreprocessor,
    },
    Tensor {
        column: String,
        input_shape: Vec<usize>,
    },
    Sample {
        column: String,
        input_shape: Vec<usize>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PreparedTableDataset {
    pub source: PinnedTableSource,
    pub spec: InspectionSpec,
    pub split: DatasetSplit,
    pub samples: Vec<Sample>,
    pub preprocessing: TablePreprocessing,
    pub profile: TableProfile,
    pub digest: String,
    #[serde(default)]
    pub rows: Arc<Vec<Value>>,
    /// Source rows before feature engineering; shares `rows` when no columns were derived.
    #[serde(default)]
    pub raw_rows: Arc<Vec<Value>>,
    #[serde(default)]
    pub annotation_revisions: BTreeMap<String, i64>,
    #[serde(default)]
    pub raw_content_digests: BTreeMap<String, String>,
    #[serde(default)]
    pub label_available_at_ms: BTreeMap<String, i64>,
    #[serde(default)]
    pub feature_sources: Vec<super::feature_engineering::PinnedFeatureSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TableWritePolicy {
    pub create_tables: bool,
    pub table_prefix: Option<String>,
    pub allowed_destination: Option<NodeDBConnection>,
    pub expected_destination: Option<DatabaseReference>,
    pub maximum_rows: usize,
    pub maximum_bytes: usize,
}
impl Default for TableWritePolicy {
    fn default() -> Self {
        Self {
            create_tables: true,
            table_prefix: None,
            allowed_destination: None,
            expected_destination: None,
            maximum_rows: 100_000,
            maximum_bytes: 256 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct MaterializedTable {
    pub partition: String,
    pub database: NodeDBConnection,
    pub reference: DatabaseReference,
    pub source: DatabaseReference,
    pub rows: usize,
    pub preprocessing_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct TableColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SourceTableSchema {
    pub source: PinnedTableSource,
    pub columns: Vec<TableColumnSchema>,
}

fn field<'a>(row: &'a Value, name: &str) -> Result<&'a Value> {
    row.as_object()
        .and_then(|row| row.get(name))
        .ok_or_else(|| anyhow!("Missing table column '{name}'"))
}
fn identifier(value: &Value) -> Result<String> {
    let result = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) if value.is_i64() || value.is_u64() => value.to_string(),
        _ => {
            return Err(anyhow!(
                "Row/group identifiers must be non-null strings or integers"
            ));
        }
    };
    if result.is_empty() || result.len() > 4096 {
        return Err(anyhow!(
            "Row/group identifier is empty or exceeds 4096 bytes"
        ));
    }
    Ok(result)
}
fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T> {
    Ok(if let Some(text) = value.as_str() {
        flow_like_types::json::from_str(text)?
    } else {
        flow_like_types::json::from_value(value.clone())?
    })
}
fn timestamp(
    row: &Value,
    column: &Option<String>,
    default: i64,
    units: &BTreeMap<String, TemporalUnit>,
) -> Result<i64> {
    let Some(column) = column else {
        return Ok(default);
    };
    let value = field(row, column)?.as_i64().ok_or_else(|| {
        anyhow!("Timestamp '{column}' must be integer epoch milliseconds or a timestamp column")
    })?;
    match units.get(column) {
        None => Ok(value),
        Some(unit) => unit.to_milliseconds(value).ok_or_else(|| {
            anyhow!("Timestamp '{column}' value {value} in {unit:?} overflows epoch milliseconds")
        }),
    }
}
fn finite(value: &Value) -> Result<f64> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| anyhow!("Numeric targets must be finite"))
}
fn target(row: &Value, spec: &InspectionSpec, mapping: &TargetSource) -> Result<Annotation> {
    match mapping {
        TargetSource::Annotation { column } => decode(field(row, column)?),
        TargetSource::Columns { columns } => {
            if columns.is_empty() || columns.len() > 65536 {
                return Err(anyhow!("Dense targets require bounded, nonempty columns"));
            }
            Ok(Annotation::Values {
                values: columns
                    .iter()
                    .map(|name| {
                        let value = finite(field(row, name)?)? as f32;
                        if !value.is_finite() {
                            return Err(anyhow!("Dense target exceeds float32 range"));
                        }
                        Ok(value)
                    })
                    .collect::<Result<_>>()?,
            })
        }
        TargetSource::Sample => Err(anyhow!("Sample targets require serialized sample features")),
        TargetSource::Column { column } => {
            let value = field(row, column)?;
            Ok(match spec.task {
                TaskKind::ImageClassification
                | TaskKind::SensorClassification
                | TaskKind::VisualSequenceClassification
                | TaskKind::Fusion
                    if !spec.labels.is_empty() =>
                {
                    let label = value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string());
                    let class_id = spec
                        .labels
                        .iter()
                        .position(|candidate| candidate == &label)
                        .or_else(|| {
                            value
                                .as_u64()
                                .and_then(|id| usize::try_from(id).ok())
                                .filter(|id| *id < spec.labels.len())
                        })
                        .ok_or_else(|| anyhow!("Unknown target label '{label}'"))?;
                    Annotation::Class {
                        class_id: class_id as u32,
                    }
                }
                TaskKind::SensorRegression | TaskKind::Fusion => Annotation::Scalar {
                    value: finite(value)?,
                },
                TaskKind::VisualAnomaly
                | TaskKind::SensorAnomaly
                | TaskKind::SequenceAutoencoder => Annotation::Anomaly {
                    is_anomaly: value
                        .as_bool()
                        .ok_or_else(|| anyhow!("Anomaly target must be a boolean"))?,
                },
                TaskKind::SequenceForecast => Annotation::Values {
                    values: decode(value)?,
                },
                _ => return Err(anyhow!("This task needs a typed annotation column")),
            })
        }
    }
}

fn validate_mapping(mapping: &TableMapping) -> Result<()> {
    if mapping.row_id.trim().is_empty() {
        return Err(anyhow!("An explicit stable row ID column is required"));
    }
    if let FeatureSource::Tabular { options } = &mapping.features {
        let mut protected: HashSet<&str> =
            [&mapping.row_id].into_iter().map(String::as_str).collect();
        for column in [
            &mapping.group_id,
            &mapping.timestamp_ms,
            &mapping.window_start_ms,
            &mapping.window_end_ms,
            &mapping.outcome,
        ]
        .into_iter()
        .flatten()
        {
            protected.insert(column);
        }
        match &mapping.target {
            TargetSource::Column { column } | TargetSource::Annotation { column } => {
                protected.insert(column);
            }
            TargetSource::Columns { columns } => {
                protected.extend(columns.iter().map(String::as_str));
            }
            TargetSource::Sample => {
                return Err(anyhow!("Tabular features need an explicit target mapping"));
            }
        }
        if options
            .numeric_columns
            .iter()
            .chain(&options.categorical_columns)
            .any(|name| protected.contains(name.as_str()))
        {
            return Err(anyhow!(
                "Feature columns must exclude row/group/time keys, outcomes and targets"
            ));
        }
    }
    Ok(())
}

/// Feature and target columns, the only source columns a training profile may describe.
pub(crate) fn profiled_columns(mapping: &TableMapping) -> Vec<String> {
    let mut columns: Vec<String> = match &mapping.features {
        FeatureSource::Tabular { options } => options
            .numeric_columns
            .iter()
            .chain(&options.categorical_columns)
            .cloned()
            .collect(),
        FeatureSource::Tensor { column } | FeatureSource::Sample { column } => {
            vec![column.clone()]
        }
    };
    match &mapping.target {
        TargetSource::Column { column } | TargetSource::Annotation { column } => {
            columns.push(column.clone())
        }
        TargetSource::Columns { columns: targets } => columns.extend(targets.iter().cloned()),
        TargetSource::Sample => {}
    }
    columns
}

/// Source columns a mapping reads: keys, timestamps, outcome, features and target.
#[cfg(any(feature = "execute", test))]
pub(crate) fn mapped_columns(mapping: &TableMapping) -> Vec<String> {
    let mut columns = vec![mapping.row_id.clone()];
    columns.extend(
        [
            &mapping.group_id,
            &mapping.timestamp_ms,
            &mapping.window_start_ms,
            &mapping.window_end_ms,
            &mapping.outcome,
        ]
        .into_iter()
        .flatten()
        .cloned(),
    );
    columns.extend(profiled_columns(mapping));
    let mut seen = HashSet::new();
    columns.retain(|column| seen.insert(column.clone()));
    columns
}

/// Split identity uses row/group/time metadata before any fitted transformation.
pub(crate) fn raw_input_digest(
    mapping: &TableMapping,
    row: &Value,
    sample: &Sample,
) -> Result<String> {
    let raw_features = match &mapping.features {
        FeatureSource::Tabular { options } => Value::Object(
            options
                .numeric_columns
                .iter()
                .chain(&options.categorical_columns)
                .map(|name| (name.clone(), row.get(name).cloned().unwrap_or(Value::Null)))
                .collect(),
        ),
        FeatureSource::Tensor { column } => field(row, column)?.clone(),
        FeatureSource::Sample { .. } => json!(sample.input),
    };
    Ok(content_digest(&flow_like_types::json::to_vec(&json!({
        "features":raw_features,"group_id":sample.group_id,"timestamp_ms":sample.timestamp_ms,
        "window_start_ms":sample.window_start_ms,"window_end_ms":sample.window_end_ms,
    }))?))
}

pub(crate) struct TableIdentity {
    pub samples: Vec<Sample>,
    pub split: DatasetSplit,
    pub raw_content_digests: BTreeMap<String, String>,
}

pub(crate) fn serialized_json_size(value: &impl Serialize) -> Result<usize> {
    Ok(flow_like_ml_native::feature_engineering::serialized_bytes(
        value,
    )?)
}

pub(crate) fn prepare_identity_rows(
    source: &PinnedTableSource,
    request: &TableDatasetRequest,
    rows: &[Value],
) -> Result<TableIdentity> {
    request.budget.validate()?;
    request.spec.validate()?;
    validate_mapping(&request.mapping)?;
    if !source.reference.pinned
        || !source.reference.read_only
        || request.stream_id.trim().is_empty()
    {
        return Err(anyhow!(
            "Preparation requires a pinned source and an inspection stream"
        ));
    }
    if rows.len() > request.budget.maximum_rows {
        return Err(anyhow!("Table exceeds the configured row budget"));
    }
    let total_bytes = rows.iter().try_fold(0usize, |size, row| -> Result<usize> {
        size.checked_add(serialized_json_size(row)?)
            .ok_or_else(|| anyhow!("Table byte count overflow"))
    })?;
    if total_bytes > request.budget.maximum_bytes {
        return Err(anyhow!("Table exceeds the configured byte budget"));
    }
    let mut seen = HashSet::new();
    let mut samples = Vec::with_capacity(rows.len());
    let mut raw_content_digests = BTreeMap::new();
    for row in rows {
        let id = identifier(field(row, &request.mapping.row_id)?)?;
        if !seen.insert(id.clone()) {
            return Err(anyhow!("Duplicate row ID '{id}'"));
        }
        let mut sample = if let FeatureSource::Sample { column } = &request.mapping.features {
            if !matches!(request.mapping.target, TargetSource::Sample) {
                return Err(anyhow!(
                    "Serialized samples preserve their own target and provenance"
                ));
            }
            let sample: Sample = decode(field(row, column)?)?;
            if sample.id != id || sample.stream_id != request.stream_id {
                return Err(anyhow!(
                    "Serialized sample identity/stream differs from the table mapping"
                ));
            }
            sample
        } else {
            let units = &source.temporal_units;
            let time = timestamp(row, &request.mapping.timestamp_ms, 0, units)?;
            if matches!(request.split, TableSplitPolicy::Time { .. })
                && request.mapping.timestamp_ms.is_none()
            {
                return Err(anyhow!(
                    "Temporal splits require a timestamp column or serialized samples"
                ));
            }
            Sample {
                id: id.clone(),
                group_id: match &request.mapping.group_id {
                    Some(column) => identifier(field(row, column)?)?,
                    None => id.clone(),
                },
                stream_id: request.stream_id.clone(),
                timestamp_ms: time,
                window_start_ms: timestamp(row, &request.mapping.window_start_ms, time, units)?,
                window_end_ms: timestamp(row, &request.mapping.window_end_ms, time, units)?,
                input: match &request.mapping.features {
                    FeatureSource::Tensor { column } => decode(field(row, column)?)?,
                    _ => TensorData {
                        shape: vec![1],
                        values: vec![0.0],
                    },
                },
                annotation: request
                    .label_overrides
                    .get(&id)
                    .map(|review| Ok(review.annotation.clone()))
                    .unwrap_or_else(|| target(row, &request.spec, &request.mapping.target))?,
                provenance: request.mapping.provenance.clone(),
                outcome: request
                    .mapping
                    .outcome
                    .as_ref()
                    .map(|column| decode::<Outcome>(field(row, column)?))
                    .transpose()?,
            }
        };
        if let Some(review) = request.label_overrides.get(&sample.id) {
            if review.annotation_revision < 1 {
                return Err(anyhow!("Label override revisions must be positive"));
            }
            if let Some(available) = review.available_at_ms {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis();
                if available < sample.timestamp_ms || available as i128 > now as i128 {
                    return Err(anyhow!(
                        "Review availability must follow capture and cannot be in the future"
                    ));
                }
            }
            sample.annotation = review.annotation.clone();
            sample.provenance = review.provenance.clone();
            sample.outcome = review.outcome.clone();
        }
        sample.validate(
            request.spec.labels.len(),
            request.budget.maximum_tensor_elements,
        )?;
        raw_content_digests.insert(
            sample.id.clone(),
            raw_input_digest(&request.mapping, row, &sample)?,
        );
        samples.push(sample);
    }
    let mut split = match request.split {
        TableSplitPolicy::Group {
            train_fraction,
            validation_fraction,
            seed,
        } => split_grouped(&samples, train_fraction, validation_fraction, seed)?,
        TableSplitPolicy::Time {
            train_end_ms,
            validation_end_ms,
            embargo_ms,
        } => split_temporal(
            &samples,
            train_end_ms,
            validation_end_ms,
            embargo_ms,
            request.spec.prediction_horizon_ms.unwrap_or(0),
        )?,
    };
    if let TableSplitPolicy::Time {
        train_end_ms,
        validation_end_ms,
        ..
    } = request.split
    {
        for (indices, deadline) in [
            (&mut split.train, train_end_ms),
            (&mut split.validation, validation_end_ms),
        ] {
            indices.retain(|index| {
                let sample = &samples[*index];
                let reviewed = match sample.provenance {
                    LabelProvenance::Reviewed { reviewed_at_ms, .. } => reviewed_at_ms,
                    _ => sample.timestamp_ms,
                };
                let available = reviewed
                    .max(
                        sample
                            .outcome
                            .as_ref()
                            .map_or(sample.timestamp_ms, |outcome| outcome.available_at_ms),
                    )
                    .max(
                        request
                            .label_overrides
                            .get(&sample.id)
                            .and_then(|review| review.available_at_ms)
                            .unwrap_or(sample.timestamp_ms),
                    );
                // Like split_temporal and the snapshot check, a label available at the cutoff is inside.
                if available > deadline {
                    split.purged.push(*index);
                    false
                } else {
                    true
                }
            });
        }
    }
    let selection = &request.selection;
    let excluded_ids: HashSet<_> = selection.excluded_ids.iter().collect();
    let excluded_groups: HashSet<_> = selection.excluded_groups.iter().collect();
    let ineligible_ids: HashSet<_> = selection.ineligible_test_ids.iter().collect();
    let ineligible_groups: HashSet<_> = selection.ineligible_test_groups.iter().collect();
    for indices in [&mut split.train, &mut split.validation, &mut split.test] {
        indices.retain(|index| {
            let sample = &samples[*index];
            let retain =
                !excluded_ids.contains(&sample.id) && !excluded_groups.contains(&sample.group_id);
            if !retain {
                split.purged.push(*index);
            }
            retain
        });
    }
    split.test.retain(|index| {
        let sample = &samples[*index];
        let retain =
            !ineligible_ids.contains(&sample.id) && !ineligible_groups.contains(&sample.group_id);
        if !retain {
            split.purged.push(*index);
        }
        retain
    });
    if split.train.is_empty() || split.validation.is_empty() || split.test.is_empty() {
        return Err(anyhow!(
            "Preparation requires nonempty train, validation and test partitions after purging"
        ));
    }
    Ok(TableIdentity {
        samples,
        split,
        raw_content_digests,
    })
}

pub fn prepare_rows(
    source: PinnedTableSource,
    request: &TableDatasetRequest,
    rows: &[Value],
) -> Result<PreparedTableDataset> {
    prepare_rows_owned(source, request, rows.to_vec())
}

/// `prepare_rows` for rows the caller hands over; `rows` and `raw_rows` share them.
pub fn prepare_rows_owned(
    source: PinnedTableSource,
    request: &TableDatasetRequest,
    rows: Vec<Value>,
) -> Result<PreparedTableDataset> {
    let TableIdentity {
        mut samples,
        split,
        raw_content_digests,
    } = prepare_identity_rows(&source, request, &rows)?;
    let preprocessing = match &request.mapping.features {
        FeatureSource::Tabular { options } => {
            let fitted = FittedTabularPreprocessor::fit(&rows, &split.train, options)?;
            let inputs = fitted.transform_rows(&rows, request.budget.maximum_tensor_elements)?;
            for (sample, input) in samples.iter_mut().zip(inputs) {
                sample.input = input;
            }
            TablePreprocessing::Tabular { fitted }
        }
        FeatureSource::Tensor { column } => TablePreprocessing::Tensor {
            column: column.clone(),
            input_shape: request.spec.input_shape.clone(),
        },
        FeatureSource::Sample { column } => TablePreprocessing::Sample {
            column: column.clone(),
            input_shape: request.spec.input_shape.clone(),
        },
    };
    let mut spec = request.spec.clone();
    if matches!(preprocessing, TablePreprocessing::Tabular { .. }) {
        spec.input_shape = samples[0].input.shape.clone();
    }
    let mut elements = 0usize;
    for sample in &samples {
        spec.validate_sample(sample)?;
        elements = elements
            .checked_add(sample.input.values.len())
            .ok_or_else(|| anyhow!("Tensor budget overflows"))?;
        if elements > request.budget.maximum_tensor_elements {
            return Err(anyhow!(
                "Prepared tensors exceed the aggregate element budget"
            ));
        }
    }
    let profile = profile_training_rows(&rows, &split.train, &profiled_columns(&request.mapping))?;
    let digest = content_digest(&flow_like_types::json::to_vec(&(
        &source.reference,
        &spec,
        &split,
        &samples,
        &preprocessing,
    ))?);
    let rows = Arc::new(rows);
    Ok(PreparedTableDataset {
        source,
        spec,
        split,
        samples,
        preprocessing,
        profile,
        digest,
        rows: Arc::clone(&rows),
        raw_rows: rows,
        annotation_revisions: request
            .label_overrides
            .iter()
            .map(|(id, review)| (id.clone(), review.annotation_revision))
            .collect(),
        raw_content_digests,
        label_available_at_ms: request
            .label_overrides
            .iter()
            .filter_map(|(id, review)| {
                review
                    .available_at_ms
                    .map(|available| (id.clone(), available))
            })
            .collect(),
        feature_sources: Vec::new(),
    })
}

/// Profiles only `columns` of training rows, so unmapped source values never reach a consultant.
pub(crate) fn profile_training_rows(
    rows: &[Value],
    training: &[usize],
    columns: &[String],
) -> Result<TableProfile> {
    let mut profiles: BTreeMap<String, ColumnProfile> = columns
        .iter()
        .map(|name| {
            (
                name.clone(),
                ColumnProfile {
                    name: name.clone(),
                    missing: 0,
                    numeric: 0,
                    boolean: 0,
                    text: 0,
                    other: 0,
                    minimum: None,
                    maximum: None,
                    examples: vec![],
                },
            )
        })
        .collect();
    if profiles.len() > 4096 {
        return Err(anyhow!("Table profile exceeds 4096 columns"));
    }
    for index in training {
        let row = rows[*index]
            .as_object()
            .ok_or_else(|| anyhow!("Table rows must be objects"))?;
        for (name, profile) in &mut profiles {
            let value = row.get(name).unwrap_or(&Value::Null);
            match value {
                Value::Null => profile.missing += 1,
                Value::Number(number) => {
                    profile.numeric += 1;
                    if let Some(value) = number.as_f64() {
                        profile.minimum = Some(
                            profile
                                .minimum
                                .map_or(value, |previous| previous.min(value)),
                        );
                        profile.maximum = Some(
                            profile
                                .maximum
                                .map_or(value, |previous| previous.max(value)),
                        );
                    }
                }
                Value::Bool(_) => profile.boolean += 1,
                Value::String(_) => profile.text += 1,
                _ => profile.other += 1,
            }
            if profile.examples.len() < 5
                && !value.is_null()
                && !profile.examples.contains(value)
                && flow_like_types::json::to_vec(value)?.len() <= 512
            {
                profile.examples.push(value.clone());
            }
        }
    }
    Ok(TableProfile {
        partition: "train".into(),
        rows: training.len(),
        columns: profiles.into_values().collect(),
    })
}

pub fn transform_table_rows(
    preprocessing: &TablePreprocessing,
    rows: &[Value],
    maximum_elements: usize,
) -> Result<Vec<TensorData>> {
    match preprocessing {
        TablePreprocessing::Engineered {
            pipeline,
            preprocessing,
            projection,
            ..
        } => {
            let inputs = transform_table_rows(
                preprocessing,
                &pipeline.transform_rows(rows)?,
                maximum_elements,
            )?;
            if let Some(projection) = projection {
                Ok(projection.transform_rows(&inputs, maximum_elements)?)
            } else {
                Ok(inputs)
            }
        }
        TablePreprocessing::Tabular { fitted } => {
            Ok(fitted.transform_rows(rows, maximum_elements)?)
        }
        TablePreprocessing::Tensor {
            column,
            input_shape,
        }
        | TablePreprocessing::Sample {
            column,
            input_shape,
        } => {
            let mut total = 0usize;
            rows.iter()
                .map(|row| {
                    let tensor = if matches!(preprocessing, TablePreprocessing::Sample { .. }) {
                        decode::<Sample>(field(row, column)?)?.input
                    } else {
                        decode::<TensorData>(field(row, column)?)?
                    };
                    tensor.validate(maximum_elements)?;
                    if &tensor.shape != input_shape {
                        return Err(anyhow!(
                            "Inference tensor shape differs from fitted preprocessing"
                        ));
                    }
                    total = total
                        .checked_add(tensor.values.len())
                        .ok_or_else(|| anyhow!("Tensor budget overflow"))?;
                    if total > maximum_elements {
                        return Err(anyhow!("Inference tensors exceed aggregate element budget"));
                    }
                    Ok(tensor)
                })
                .collect()
        }
    }
}

pub fn transform_table_rows_with_state(
    preprocessing: &TablePreprocessing,
    rows: &[Value],
    maximum_elements: usize,
    state: Option<flow_like_ml_native::feature_engineering::FeatureStreamState>,
    stateful: bool,
) -> Result<(
    Vec<TensorData>,
    Option<flow_like_ml_native::feature_engineering::FeatureStreamState>,
)> {
    if let TablePreprocessing::Engineered {
        pipeline,
        preprocessing,
        projection,
        ..
    } = preprocessing
    {
        if stateful || state.is_some() {
            if rows.len() > pipeline.limits.maximum_rows {
                return Err(anyhow!("Feature batch exceeds the row budget"));
            }
            let mut stream = if let Some(state) = state {
                pipeline.stream_with_state(state)?
            } else {
                pipeline.stream()?
            };
            let started = std::time::Instant::now();
            let mut bytes = 0usize;
            let mut output_bytes = 0usize;
            let transformed = rows
                .iter()
                .map(|row| -> Result<Value> {
                    bytes = bytes
                        .checked_add(serialized_json_size(row)?)
                        .ok_or_else(|| anyhow!("Feature byte budget overflow"))?;
                    if bytes > pipeline.limits.maximum_bytes
                        || started.elapsed().as_millis()
                            > pipeline.limits.maximum_duration_ms as u128
                    {
                        return Err(anyhow!("Feature batch exceeds its byte or time budget"));
                    }
                    let output = stream.transform_row(row)?;
                    output_bytes = output_bytes
                        .checked_add(serialized_json_size(&output)?)
                        .ok_or_else(|| anyhow!("Feature byte budget overflow"))?;
                    if output_bytes > pipeline.limits.maximum_bytes {
                        return Err(anyhow!("Transformed feature batch exceeds the byte budget"));
                    }
                    Ok(output)
                })
                .collect::<Result<Vec<_>>>()?;
            let inputs = transform_table_rows(preprocessing, &transformed, maximum_elements)?;
            let inputs = if let Some(projection) = projection {
                projection.transform_rows(&inputs, maximum_elements)?
            } else {
                inputs
            };
            return Ok((inputs, Some(stream.state())));
        }
    } else if state.is_some() || stateful {
        return Err(anyhow!(
            "Feature state requires an engineered model preprocessing pipeline"
        ));
    }
    Ok((
        transform_table_rows(preprocessing, rows, maximum_elements)?,
        None,
    ))
}

pub fn base_preprocessing(preprocessing: &TablePreprocessing) -> &TablePreprocessing {
    match preprocessing {
        TablePreprocessing::Engineered { preprocessing, .. } => base_preprocessing(preprocessing),
        preprocessing => preprocessing,
    }
}

#[cfg(feature = "execute")]
mod table_io {
    use super::*;
    use flow_like::flow::execution::context::{ExecutionContext, ExecutionContextCache};
    use flow_like_catalog_core::{CachedDB, CachedDBRefreshHook, CachedDBRefresher};
    use flow_like_storage::{
        arrow_schema::{DataType, Schema, TimeUnit},
        arrow_utils::value_to_record_batch,
        contracts::database::DatabaseSelector,
        databases::vector::{
            VectorStore, buffered::BufferedVectorStore, lancedb::LanceDBVectorStore,
        },
        lancedb::Connection,
    };
    use flow_like_types::{async_trait, sync::RwLock};
    use std::{
        collections::HashMap,
        sync::{
            LazyLock, Mutex, Weak,
            atomic::{AtomicU64, Ordering},
        },
    };

    /// Upper bound on rows per read query; byte-sized pages are usually smaller.
    const MAXIMUM_PAGE_ROWS: usize = 4096;

    struct ViewRefresher {
        source: NodeDBConnection,
        cache_key: String,
        source_generation: AtomicU64,
        generation: AtomicU64,
        lock: tokio::sync::Mutex<()>,
    }
    #[async_trait]
    impl CachedDBRefresher for ViewRefresher {
        async fn refresh(&self, context: &ExecutionContext) -> Result<()> {
            let _lock = self.lock.lock().await;
            let (source, generation) = self
                .source
                .load_with_generation(&mut context.clone())
                .await?;
            if self.source_generation.load(Ordering::Acquire) == generation {
                return Ok(());
            }
            let value = context
                .cache
                .read()
                .await
                .get(&self.cache_key)
                .cloned()
                .ok_or_else(|| anyhow!("Prepared table cache disappeared"))?;
            let cached = value
                .as_any()
                .downcast_ref::<CachedDB>()
                .ok_or_else(|| anyhow!("Unexpected prepared table cache type"))?;
            let mut target = cached.db.write().await;
            let source = source.db.read().await;
            let replacement = target
                .inner()
                .reopen(source.inner().connection()?.clone())
                .await?;
            *target.inner_mut() = replacement;
            self.source_generation.store(generation, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
            Ok(())
        }
        fn generation(&self) -> u64 {
            self.generation.load(Ordering::Acquire)
        }
    }
    async fn cache_view(
        context: &mut ExecutionContext,
        source: NodeDBConnection,
        store: LanceDBVectorStore,
    ) -> Result<NodeDBConnection> {
        let (_, generation) = source.load_with_generation(context).await?;
        let cache_key = format!("inspection::table::{}", uuid::Uuid::new_v4());
        let cached = CachedDB {
            db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
        };
        let refresher = Arc::new(ViewRefresher {
            source,
            cache_key: cache_key.clone(),
            source_generation: AtomicU64::new(generation),
            generation: AtomicU64::new(0),
            lock: tokio::sync::Mutex::new(()),
        });
        let mut cache = context.cache.write().await;
        cache.insert(cache_key.clone(), Arc::new(cached));
        cache.insert(
            NodeDBConnection::refresh_cache_key(&cache_key),
            Arc::new(CachedDBRefreshHook::new(refresher)),
        );
        Ok(NodeDBConnection { cache_key })
    }
    pub async fn pin_table_source(
        context: &mut ExecutionContext,
        source: &NodeDBConnection,
    ) -> Result<PinnedTableSource> {
        let cached = source.load(context).await?;
        cached.ensure_flushed().await?;
        let store = cached.db.read().await.inner().clone();
        let scope = context
            .execution_cache
            .as_ref()
            .ok_or_else(|| anyhow!("Execution storage scope is unavailable"))?;
        let locator = TableLocator {
            app_id: scope.app_id.clone(),
            board_id: scope.board_id.clone(),
            connection_digest: content_digest(store.connection()?.uri().as_bytes()),
        };
        let reference = store.reference().await?;
        let snapshot = store
            .checkout(DatabaseSelector {
                branch: reference.branch,
                version: Some(reference.version),
                tag: None,
                read_only: true,
            })
            .await?;
        let reference = snapshot.reference().await?;
        let temporal_units = schema_temporal_units(&VectorStore::schema(&snapshot).await?);
        let database = cache_view(context, source.clone(), snapshot).await?;
        Ok(PinnedTableSource {
            database,
            reference,
            locator: Some(locator),
            temporal_units,
        })
    }
    /// Lance returns timestamp and date cells as integers in their storage unit.
    fn schema_temporal_units(schema: &Schema) -> BTreeMap<String, TemporalUnit> {
        schema
            .fields()
            .iter()
            .filter_map(|field| {
                let unit = match field.data_type() {
                    DataType::Timestamp(TimeUnit::Second, _) => TemporalUnit::Seconds,
                    DataType::Timestamp(TimeUnit::Microsecond, _) => TemporalUnit::Microseconds,
                    DataType::Timestamp(TimeUnit::Nanosecond, _) => TemporalUnit::Nanoseconds,
                    DataType::Date32 => TemporalUnit::Days,
                    _ => return None,
                };
                Some((field.name().clone(), unit))
            })
            .collect()
    }
    pub async fn inspect_table_schema(
        context: &mut ExecutionContext,
        source: &NodeDBConnection,
    ) -> Result<SourceTableSchema> {
        let source = pin_table_source(context, source).await?;
        let cached = source.database.load(context).await?;
        let schema = cached.db.read().await.schema().await?;
        let columns = schema
            .fields()
            .iter()
            .map(|field| TableColumnSchema {
                name: field.name().clone(),
                data_type: format!("{:?}", field.data_type()),
                nullable: field.is_nullable(),
            })
            .collect();
        Ok(SourceTableSchema { source, columns })
    }
    /// Reads the pinned table in pages sized from the largest row seen so far. Unfiltered
    /// list queries let Lance push each page's offset into the scan instead of rescanning.
    async fn read_pages(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
        budget: &TableBudget,
        select: Option<Vec<String>>,
        maximum_page_rows: usize,
    ) -> Result<Vec<Value>> {
        budget.validate()?;
        let table = &source.reference.table;
        let total = source
            .database
            .load(context)
            .await?
            .db
            .read()
            .await
            .count(None)
            .await?;
        if total > budget.maximum_rows {
            return Err(anyhow!(
                "Source table '{table}' has {total} rows, above the row budget of {}; no partial dataset was prepared",
                budget.maximum_rows
            ));
        }
        let page_bytes = (budget.maximum_bytes / 16).max(1);
        let mut rows = Vec::with_capacity(total);
        let mut bytes = 0usize;
        let mut largest = 0usize;
        loop {
            let page_rows = if largest == 0 {
                1
            } else {
                (page_bytes / largest).clamp(1, maximum_page_rows.max(1))
            };
            let limit = page_rows.min(
                budget
                    .maximum_rows
                    .saturating_add(1)
                    .saturating_sub(rows.len()),
            );
            let cached = source.database.load(context).await?;
            let page = cached
                .db
                .read()
                .await
                .list(select.clone(), limit, rows.len())
                .await?;
            if page.is_empty() {
                break;
            }
            for row in page {
                let size = serialized_json_size(&row)?;
                largest = largest.max(size);
                bytes = bytes
                    .checked_add(size)
                    .ok_or_else(|| anyhow!("Table byte budget overflows"))?;
                if bytes > budget.maximum_bytes || rows.len() >= budget.maximum_rows {
                    return Err(anyhow!(
                        "Source table '{table}' exceeds the budget of {} rows or {} bytes; no partial dataset was prepared",
                        budget.maximum_rows,
                        budget.maximum_bytes
                    ));
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }
    /// Reads every column; `page_size` caps the rows of one query.
    pub async fn read_rows(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
        budget: &TableBudget,
        page_size: usize,
    ) -> Result<Vec<Value>> {
        read_pages(context, source, budget, None, page_size).await
    }
    /// Reads only the named columns present in the pinned schema. A missing mapped column
    /// is reported by name when a row is mapped.
    pub async fn read_columns(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
        budget: &TableBudget,
        columns: &[String],
    ) -> Result<Vec<Value>> {
        if columns.is_empty() {
            return Ok(Vec::new());
        }
        let schema = source
            .database
            .load(context)
            .await?
            .db
            .read()
            .await
            .schema()
            .await?;
        let available: HashSet<&str> = schema
            .fields()
            .iter()
            .map(|field| field.name().as_str())
            .collect();
        let mut seen = HashSet::new();
        let selected: Vec<String> = columns
            .iter()
            .filter(|column| available.contains(column.as_str()) && seen.insert(column.as_str()))
            .cloned()
            .collect();
        if selected.is_empty() {
            return Err(anyhow!(
                "Table '{}' has none of the mapped columns {columns:?}",
                source.reference.table
            ));
        }
        if selected.iter().any(|column| column.contains(['.', '`'])) {
            // Lance resolves dotted names as nested field paths, so such tables are read whole.
            let mut rows = read_pages(context, source, budget, None, MAXIMUM_PAGE_ROWS).await?;
            let wanted: HashSet<&str> = selected.iter().map(String::as_str).collect();
            for row in &mut rows {
                if let Some(row) = row.as_object_mut() {
                    row.retain(|name, _| wanted.contains(name.as_str()));
                }
            }
            return Ok(rows);
        }
        read_pages(context, source, budget, Some(selected), MAXIMUM_PAGE_ROWS).await
    }
    pub async fn prepare_table_dataset(
        context: &mut ExecutionContext,
        request: &TableDatasetRequest,
    ) -> Result<PreparedTableDataset> {
        let source = pin_table_source(context, &request.source).await?;
        let columns = mapped_columns(&request.mapping);
        let rows = read_columns(context, &source, &request.budget, &columns).await?;
        let request = request.clone();
        tokio::task::spawn_blocking(move || prepare_rows_owned(source, &request, rows)).await?
    }
    pub async fn clone_experiment_table(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
        name: &str,
    ) -> Result<PinnedTableSource> {
        if context
            .execution_cache
            .as_ref()
            .is_some_and(|scope| scope.shadow)
        {
            return Err(anyhow!("Shadow execution cannot create experiment tables"));
        }
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || name == source.reference.table
        {
            return Err(anyhow!(
                "Clone requires a new bounded alphanumeric table name"
            ));
        }
        let rows = read_rows(context, source, &TableBudget::default(), MAXIMUM_PAGE_ROWS).await?;
        let cached = source.database.load(context).await?;
        let connection = cached.db.read().await.inner().connection()?.clone();
        let mut store = LanceDBVectorStore::from_connection(connection, name.into()).await;
        if store.table_exists().await? {
            return Err(anyhow!("Clone destination already exists"));
        }
        store.insert(rows).await?;
        let reference = store.reference().await?;
        let database = cache_view(context, source.database.clone(), store).await?;
        Ok(PinnedTableSource {
            database,
            reference,
            locator: source.locator.clone(),
            temporal_units: source.temporal_units.clone(),
        })
    }
    fn authorized_locator<'a>(
        context: &ExecutionContext,
        source: &'a PinnedTableSource,
    ) -> Result<(&'a TableLocator, ExecutionContextCache)> {
        let locator = source.locator.as_ref().ok_or_else(|| {
            anyhow!(
                "Durable source locator is missing; supply the original scoped database connection"
            )
        })?;
        let scope = context
            .execution_cache
            .as_ref()
            .ok_or_else(|| anyhow!("Execution storage scope is unavailable"))?
            .clone();
        if locator.app_id != scope.app_id
            || locator.board_id != scope.board_id
            || !source.reference.pinned
            || !source.reference.read_only
        {
            return Err(anyhow!(
                "Dataset source belongs to a different execution scope or is not pinned"
            ));
        }
        Ok((locator, scope))
    }
    /// The pinned store this run already holds, checked against the durable reference.
    async fn cached_source(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
        locator: &TableLocator,
    ) -> Result<Option<LanceDBVectorStore>> {
        let cache_present = context
            .cache
            .read()
            .await
            .contains_key(&source.database.cache_key);
        if !cache_present {
            return Ok(None);
        }
        let cached = source.database.load(context).await?;
        let store = cached.db.read().await.inner().clone();
        if store.reference().await? != source.reference
            || content_digest(store.connection()?.uri().as_bytes()) != locator.connection_digest
        {
            return Err(anyhow!(
                "Cached source differs from the pinned dataset reference"
            ));
        }
        Ok(Some(store))
    }
    async fn cache_reopened(
        context: &ExecutionContext,
        store: LanceDBVectorStore,
    ) -> NodeDBConnection {
        let cache_key = format!("inspection::reopened::{}", uuid::Uuid::new_v4());
        context.cache.write().await.insert(
            cache_key.clone(),
            Arc::new(CachedDB {
                db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
            }),
        );
        NodeDBConnection { cache_key }
    }
    pub async fn reopen_pinned_source(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
    ) -> Result<NodeDBConnection> {
        let (locator, scope) = authorized_locator(context, source)?;
        if cached_source(context, source, locator).await?.is_some() {
            return Ok(source.database.clone());
        }
        let connection = locate_source_connection(context, &scope, locator).await?;
        let selector = DatabaseSelector {
            branch: source.reference.branch.clone(),
            version: Some(source.reference.version),
            tag: None,
            read_only: true,
        };
        let store = match LanceDBVectorStore::from_connection_with_selector(
            connection.clone(),
            source.reference.table.clone(),
            selector,
        )
        .await
        {
            Ok(store) => store,
            Err(error) => return Err(missing_version_error(connection, source, error).await),
        };
        if store.reference().await? != source.reference {
            return Err(anyhow!(
                "Reopened source differs from the pinned dataset reference"
            ));
        }
        Ok(cache_reopened(context, store).await)
    }
    /// Names a cleaned-up pinned version instead of surfacing Lance's checkout error.
    async fn missing_version_error(
        connection: Connection,
        source: &PinnedTableSource,
        error: flow_like_types::Error,
    ) -> flow_like_types::Error {
        let reference = &source.reference;
        let head = LanceDBVectorStore::from_connection_with_selector(
            connection,
            reference.table.clone(),
            DatabaseSelector {
                branch: reference.branch.clone(),
                version: None,
                tag: None,
                read_only: true,
            },
        )
        .await;
        let versions = match head {
            Ok(head) => head.list_versions().await,
            Err(_) => return error,
        };
        match versions {
            Ok(versions)
                if versions
                    .iter()
                    .all(|kept| kept.version != reference.version) =>
            {
                anyhow!(
                    "Pinned version {} of table '{}' on branch '{}' was cleaned up or the table was recreated, so this exact replay cannot run; prepare the dataset again from the current table ({error})",
                    reference.version,
                    reference.table,
                    reference.branch
                )
            }
            _ => error,
        }
    }
    /// Finds the project or user database whose URI digest the durable locator recorded.
    async fn locate_source_connection(
        context: &ExecutionContext,
        scope: &ExecutionContextCache,
        locator: &TableLocator,
    ) -> Result<Connection> {
        let callbacks = context.app_state.config.read().await.callbacks.clone();
        let mut matched = None;
        for user_scoped in [false, true] {
            let path = if user_scoped {
                scope.get_user_dir(false)?
            } else {
                scope.get_storage(false)?
            }
            .join("db");
            let builder = if let Some(credentials) = &context.credentials {
                if user_scoped {
                    credentials.to_db_scoped(&scope.sub, &scope.app_id).await
                } else {
                    credentials.to_db(&scope.app_id).await
                }
            } else {
                match if user_scoped {
                    callbacks.build_user_database.as_ref()
                } else {
                    callbacks.build_project_database.as_ref()
                } {
                    Some(build) => Ok(build(path)),
                    None => continue,
                }
            };
            let Ok(builder) = builder else {
                continue;
            };
            let Ok(connection) = context
                .app_state
                .with_lance_session(builder)
                .execute()
                .await
            else {
                continue;
            };
            if content_digest(connection.uri().as_bytes()) != locator.connection_digest {
                continue;
            }
            matched = Some(connection);
            break;
        }
        matched.ok_or_else(|| {
            anyhow!(
                "The source connection is not available in the current authorized workspace; reconnect the original database"
            )
        })
    }
    /// Opens the head of the pinned table's branch without opening the pinned version,
    /// which version cleanup may have removed.
    pub async fn reopen_latest_source(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
    ) -> Result<NodeDBConnection> {
        let (locator, scope) = authorized_locator(context, source)?;
        let head = DatabaseSelector {
            branch: source.reference.branch.clone(),
            version: None,
            tag: None,
            read_only: true,
        };
        if let Some(store) = cached_source(context, source, locator).await? {
            let latest = store.checkout(head).await?;
            return cache_view(context, source.database.clone(), latest).await;
        }
        let connection = locate_source_connection(context, &scope, locator).await?;
        let latest = LanceDBVectorStore::from_connection_with_selector(
            connection,
            source.reference.table.clone(),
            head,
        )
        .await?;
        Ok(cache_reopened(context, latest).await)
    }
    fn table_name(
        experiment_id: &str,
        partition: &str,
        policy: &TableWritePolicy,
    ) -> Result<String> {
        let prefix = policy.table_prefix.as_deref().unwrap_or("ml_experiment");
        if prefix.is_empty()
            || prefix.len() > 48
            || experiment_id.trim().is_empty()
            || experiment_id.len() > 1024
            || partition.is_empty()
            || partition.len() > 32
            || !prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || !partition
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(anyhow!(
                "Experiment table names require a bounded alphanumeric prefix"
            ));
        }
        let identity = content_digest(experiment_id.as_bytes());
        Ok(format!("{prefix}_{}_{partition}", &identity[..16]))
    }
    /// Serializes materialization of one experiment table within this process.
    async fn lock_experiment_table(key: String) -> tokio::sync::OwnedMutexGuard<()> {
        static LOCKS: LazyLock<Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>> =
            LazyLock::new(|| Mutex::new(HashMap::new()));
        let lock = {
            let mut locks = LOCKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            locks.retain(|_, lock| lock.strong_count() > 0);
            let current = locks.get(&key).and_then(Weak::upgrade);
            current.unwrap_or_else(|| {
                let lock = Arc::new(tokio::sync::Mutex::new(()));
                locks.insert(key, Arc::downgrade(&lock));
                lock
            })
        };
        lock.lock_owned().await
    }
    /// Creates the table with every row in one commit. Returns false without writing when
    /// the table exists, so a concurrent writer never appends a second copy.
    pub(super) async fn create_experiment_table(
        connection: &Connection,
        name: &str,
        rows: &[Value],
    ) -> Result<bool> {
        let batch = value_to_record_batch(rows.to_vec())?;
        match connection.create_table(name, batch).execute().await {
            Ok(_) => Ok(true),
            Err(flow_like_storage::lancedb::Error::TableAlreadyExists { .. }) => Ok(false),
            Err(error) => Err(anyhow!(
                "Creating experiment table '{name}' failed: {error}"
            )),
        }
    }
    /// Pins the existing table and requires exactly the requested rows.
    async fn verify_experiment_table(
        store: &LanceDBVectorStore,
        rows: &[Value],
        policy: &TableWritePolicy,
        largest_row: usize,
    ) -> Result<LanceDBVectorStore> {
        let reference = store.reference().await?;
        let pinned = store
            .checkout(DatabaseSelector {
                branch: reference.branch,
                version: Some(reference.version),
                tag: None,
                read_only: true,
            })
            .await?;
        let expected: BTreeMap<_, _> = rows
            .iter()
            .map(|row| Ok((identifier(field(row, "sample_id")?)?, row)))
            .collect::<Result<_>>()?;
        let mut found = HashSet::new();
        let mut read_bytes = 0usize;
        let page_size = (policy.maximum_bytes / 16 / largest_row).clamp(1, 512);
        loop {
            let limit = page_size.min(rows.len().saturating_add(1).saturating_sub(found.len()));
            let page = pinned.list(None, limit, found.len()).await?;
            if page.is_empty() {
                break;
            }
            for row in page {
                read_bytes = read_bytes
                    .checked_add(serialized_json_size(&row)?)
                    .ok_or_else(|| anyhow!("Existing output byte count overflows"))?;
                let id = identifier(field(&row, "sample_id")?)?;
                if read_bytes > policy.maximum_bytes
                    || !found.insert(id.clone())
                    || expected.get(&id).is_none_or(|value| **value != row)
                {
                    return Err(anyhow!(
                        "Existing scoped experiment table '{}' differs from the requested rows at sample '{id}'; source data was not changed",
                        store.table_name()
                    ));
                }
            }
        }
        if found.len() != rows.len() {
            return Err(anyhow!(
                "Existing scoped experiment table '{}' holds {} of {} requested rows",
                store.table_name(),
                found.len(),
                rows.len()
            ));
        }
        Ok(pinned)
    }
    pub async fn write_experiment_rows(
        context: &mut ExecutionContext,
        source: &PinnedTableSource,
        rows: Vec<Value>,
        experiment_id: &str,
        partition: &str,
        preprocessing_digest: &str,
        policy: &TableWritePolicy,
    ) -> Result<MaterializedTable> {
        if context
            .execution_cache
            .as_ref()
            .is_some_and(|scope| scope.shadow)
        {
            return Err(anyhow!("Shadow execution cannot write experiment tables"));
        }
        if rows.is_empty()
            || rows.len() > policy.maximum_rows
            || policy.maximum_rows > 1_000_000
            || policy.maximum_bytes > 1024 * 1024 * 1024
        {
            return Err(anyhow!("Invalid experiment table row/write budget"));
        }
        let mut bytes = 0usize;
        let mut largest_row = 1usize;
        let mut ids = HashSet::new();
        for row in &rows {
            if !ids.insert(identifier(field(row, "sample_id")?)?) {
                return Err(anyhow!("Experiment rows require unique sample_id values"));
            }
            let row_bytes = flow_like_types::json::to_vec(row)?.len();
            largest_row = largest_row.max(row_bytes);
            bytes = bytes
                .checked_add(row_bytes)
                .ok_or_else(|| anyhow!("Write byte count overflows"))?;
            if bytes > policy.maximum_bytes {
                return Err(anyhow!("Experiment table exceeds write byte budget"));
            }
        }
        let database = if let Some(destination) = &policy.allowed_destination {
            let expected = policy.expected_destination.as_ref().ok_or_else(|| {
                anyhow!("Existing table writes require its expected durable reference")
            })?;
            let cached = destination.load(context).await?;
            cached.ensure_flushed().await?;
            let store = cached.db.read().await;
            store.inner().ensure_writable()?;
            let actual = store.inner().reference().await?;
            if &actual != expected {
                return Err(anyhow!("Destination changed since write authorization"));
            }
            let escaped = "sample_id";
            let duplicate=store.inner().sql("destination",&format!("SELECT {escaped} FROM destination GROUP BY {escaped} HAVING {escaped} IS NULL OR COUNT(*) > 1 LIMIT 1")).await?.collect().await?;
            if duplicate.iter().any(|batch| batch.num_rows() > 0) {
                return Err(anyhow!(
                    "Destination sample_id values must be unique and non-null"
                ));
            }
            drop(store);
            cached
                .upsert_from(context, rows.clone(), "sample_id".into())
                .await?;
            cached.ensure_flushed().await?;
            destination.clone()
        } else {
            if !policy.create_tables {
                return Err(anyhow!("No experiment table writes are permitted"));
            }
            let cached = source.database.load(context).await?;
            let connection = cached.db.read().await.inner().connection()?.clone();
            let name = table_name(experiment_id, partition, policy)?;
            let _serialized = lock_experiment_table(format!(
                "{}/{name}",
                content_digest(connection.uri().as_bytes())
            ))
            .await;
            let existing =
                LanceDBVectorStore::from_connection(connection.clone(), name.clone()).await;
            let created = !existing.table_exists().await?
                && create_experiment_table(&connection, &name, &rows).await?;
            let store = LanceDBVectorStore::from_connection(connection, name).await;
            let store = if created {
                store
            } else {
                verify_experiment_table(&store, &rows, policy, largest_row).await?
            };
            cache_view(context, source.database.clone(), store).await?
        };
        let cached = database.load(context).await?;
        let store = cached.db.read().await.inner().clone();
        let reference = store.reference().await?;
        let pinned = store
            .checkout(DatabaseSelector {
                branch: reference.branch,
                version: Some(reference.version),
                tag: None,
                read_only: true,
            })
            .await?;
        let reference = pinned.reference().await?;
        let database = cache_view(context, database, pinned).await?;
        Ok(MaterializedTable {
            partition: partition.into(),
            database,
            reference,
            source: source.reference.clone(),
            rows: rows.len(),
            preprocessing_digest: preprocessing_digest.into(),
        })
    }
    pub async fn materialize_prepared_tables(
        context: &mut ExecutionContext,
        prepared: &PreparedTableDataset,
        experiment_id: &str,
        policy: &TableWritePolicy,
    ) -> Result<Vec<MaterializedTable>> {
        if !policy.create_tables && policy.allowed_destination.is_none() {
            return Ok(vec![]);
        }
        prepared.split.validate(&prepared.samples)?;
        let preprocessing = flow_like_types::json::to_string(&prepared.preprocessing)?;
        let preprocessing_digest = content_digest(preprocessing.as_bytes());
        let source = flow_like_types::json::to_string(&prepared.source.reference)?;
        let groups = [
            ("train", &prepared.split.train),
            ("validation", &prepared.split.validation),
            ("test", &prepared.split.test),
        ];
        let mut tables = Vec::new();
        let mut combined = Vec::new();
        for (partition, indices) in groups {
            let rows=indices.iter().map(|index|{
                let sample=&prepared.samples[*index];
                let mut row = prepared.rows.get(*index).and_then(Value::as_object).cloned().unwrap_or_default();
                if row.get("sample_id").is_some_and(|value|!identifier(value).is_ok_and(|id|id == sample.id)) {
                    return Err(anyhow!("Source sample_id differs from the mapped stable row ID"));
                }
                row.entry("sample_id".to_owned()).or_insert_with(||json!(sample.id));
                let metadata=json!({"group_id":sample.group_id,"timestamp_ms":sample.timestamp_ms,"partition":partition,"input":sample.input.values,"shape":sample.input.shape,"sample":flow_like_types::json::to_string(sample)?,"source_reference":source,"preprocessing_digest":preprocessing_digest,"experiment_id":experiment_id});
                for (name,value) in metadata.as_object().expect("metadata is an object") {
                    let name = if row.contains_key(name) {format!("__ml_{name}")}else{name.clone()};
                    if row.insert(name,value.clone()).is_some() {return Err(anyhow!("Source columns collide with reserved training metadata"));}
                }
                Ok(Value::Object(row))
            }).collect::<Result<Vec<_>>>()?;
            if policy.allowed_destination.is_some() {
                combined.extend(rows);
            } else {
                tables.push(
                    write_experiment_rows(
                        context,
                        &prepared.source,
                        rows,
                        experiment_id,
                        partition,
                        &preprocessing_digest,
                        policy,
                    )
                    .await?,
                );
            }
        }
        if policy.allowed_destination.is_some() {
            tables.push(
                write_experiment_rows(
                    context,
                    &prepared.source,
                    combined,
                    experiment_id,
                    "prepared",
                    &preprocessing_digest,
                    policy,
                )
                .await?,
            );
        }
        Ok(tables)
    }
}
#[cfg(feature = "execute")]
pub use table_io::{
    clone_experiment_table, inspect_table_schema, materialize_prepared_tables, pin_table_source,
    prepare_table_dataset, read_columns, read_rows, reopen_latest_source, reopen_pinned_source,
    write_experiment_rows,
};

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_ml_native::preprocessing::Imputation;
    fn fixture() -> (PinnedTableSource, TableDatasetRequest, Vec<Value>) {
        let source = PinnedTableSource {
            database: NodeDBConnection {
                cache_key: "source".into(),
            },
            reference: DatabaseReference {
                table: "data".into(),
                branch: "main".into(),
                version: 1,
                read_only: true,
                pinned: true,
            },
            locator: None,
            temporal_units: BTreeMap::new(),
        };
        let request = TableDatasetRequest {
            source: source.database.clone(),
            stream_id: "inspection".into(),
            spec: InspectionSpec {
                id: "task".into(),
                description: "Classify measurements".into(),
                task: TaskKind::SensorClassification,
                labels: vec!["good".into(), "bad".into()],
                input_shape: vec![1],
                prediction_horizon_ms: None,
                minimum_examples: 3,
                minimum_examples_per_class: 1,
            },
            mapping: TableMapping {
                row_id: "id".into(),
                group_id: Some("group".into()),
                timestamp_ms: Some("time".into()),
                window_start_ms: None,
                window_end_ms: None,
                outcome: None,
                features: FeatureSource::Tabular {
                    options: TabularOptions {
                        numeric_columns: vec!["x".into()],
                        categorical_columns: vec!["category".into()],
                        imputation: Imputation::Median,
                        standardize: true,
                        maximum_categories_per_column: 10,
                        maximum_output_features: 20,
                    },
                },
                target: TargetSource::Column {
                    column: "target".into(),
                },
                provenance: LabelProvenance::Measured {
                    source: "test sensor".into(),
                },
            },
            split: TableSplitPolicy::Group {
                train_fraction: 0.6,
                validation_fraction: 0.2,
                seed: 42,
            },
            budget: TableBudget::default(),
            selection: DatasetSelection::default(),
            label_overrides: BTreeMap::new(),
        };
        let rows=(0..30).map(|index|json!({"id":index,"group":index/2,"time":index,"x":index,"z":index*2,"category":"shared","target":if index%2==0 {"good"}else{"bad"}})).collect();
        (source, request, rows)
    }
    #[test]
    fn feature_selection_preserves_split_and_held_out_rows_do_not_fit_or_profile() {
        let (source, mut request, mut rows) = fixture();
        let original = prepare_rows(source.clone(), &request, &rows).unwrap();
        for index in original.split.validation.iter().chain(&original.split.test) {
            rows[*index]["x"] = json!(1_000_000);
            rows[*index]["category"] = json!("test-only");
        }
        let changed = prepare_rows(source.clone(), &request, &rows).unwrap();
        assert_eq!(
            flow_like_types::json::to_value(&original.preprocessing).unwrap(),
            flow_like_types::json::to_value(&changed.preprocessing).unwrap()
        );
        assert_eq!(
            flow_like_types::json::to_value(&original.profile).unwrap(),
            flow_like_types::json::to_value(&changed.profile).unwrap()
        );
        let FeatureSource::Tabular { options } = &mut request.mapping.features else {
            unreachable!()
        };
        options.numeric_columns = vec!["z".into()];
        let subset = prepare_rows(source, &request, &rows).unwrap();
        assert_eq!(subset.split, original.split);
        subset.split.validate(&subset.samples).unwrap();
        assert_eq!(changed.profile.partition, "train");
    }
    #[test]
    fn target_leakage_duplicate_keys_and_partial_reads_are_rejected() {
        let (source, mut request, mut rows) = fixture();
        let FeatureSource::Tabular { options } = &mut request.mapping.features else {
            unreachable!()
        };
        options.numeric_columns = vec!["target".into()];
        assert!(prepare_rows(source.clone(), &request, &rows).is_err());
        let (_, request, _) = fixture();
        rows[1]["id"] = rows[0]["id"].clone();
        assert!(prepare_rows(source.clone(), &request, &rows).is_err());
        let (_, mut request, rows) = fixture();
        request.budget.maximum_rows = 3;
        assert!(prepare_rows(source, &request, &rows).is_err());
    }
    #[test]
    fn temporal_splits_purge_groups_and_future_outcomes() {
        let (source, mut request, mut rows) = fixture();
        request.split = TableSplitPolicy::Time {
            train_end_ms: 10,
            validation_end_ms: 20,
            embargo_ms: 1,
        };
        request.spec.task = TaskKind::SequenceForecast;
        request.spec.labels.clear();
        request.spec.prediction_horizon_ms = Some(2);
        request.mapping.target = TargetSource::Columns {
            columns: vec!["target".into()],
        };
        request.mapping.outcome = Some("outcome".into());
        for (index, row) in rows.iter_mut().enumerate() {
            row["target"] = json!(index + 2);
            row["outcome"] = json!({"target_start_ms":index+2,"target_end_ms":index+2,"available_at_ms":index+2});
        }
        let prepared = prepare_rows(source, &request, &rows).unwrap();
        assert!(prepared.split.train.iter().all(|index| {
            prepared.samples[*index]
                .outcome
                .as_ref()
                .unwrap()
                .available_at_ms
                <= 10
        }));
        assert!(prepared.split.validation.iter().all(|index| {
            prepared.samples[*index].window_start_ms > 11
                && prepared.samples[*index]
                    .outcome
                    .as_ref()
                    .unwrap()
                    .available_at_ms
                    <= 20
        }));
        assert!(prepared.split.train.contains(&8));
        assert!(!prepared.split.purged.is_empty());
        prepared.split.validate(&prepared.samples).unwrap();
    }

    #[test]
    fn review_overrides_available_after_a_cutoff_are_purged_and_at_it_are_kept() {
        let (source, mut request, rows) = fixture();
        request.split = TableSplitPolicy::Time {
            train_end_ms: 10,
            validation_end_ms: 20,
            embargo_ms: 0,
        };
        let review = |available_at_ms: i64| TableLabelOverride {
            annotation: Annotation::Class { class_id: 0 },
            provenance: LabelProvenance::Measured {
                source: "gauge".into(),
            },
            annotation_revision: 2,
            outcome: None,
            available_at_ms: Some(available_at_ms),
        };
        request.label_overrides.insert("4".into(), review(10));
        request.label_overrides.insert("2".into(), review(11));
        let prepared = prepare_rows(source, &request, &rows).unwrap();
        assert!(prepared.split.train.contains(&4));
        assert!(prepared.split.purged.contains(&2));
        assert!(!prepared.split.train.contains(&2));
    }

    #[test]
    fn temporal_columns_in_other_units_become_millisecond_sample_times() {
        let (mut source, mut request, mut rows) = fixture();
        request.split = TableSplitPolicy::Time {
            train_end_ms: 10_000,
            validation_end_ms: 20_000,
            embargo_ms: 0,
        };
        for (index, row) in rows.iter_mut().enumerate() {
            row["time"] = json!(index as i64 * 1_000_000 + 999);
        }
        source
            .temporal_units
            .insert("time".into(), TemporalUnit::Microseconds);
        let prepared = prepare_rows(source.clone(), &request, &rows).unwrap();
        for (index, sample) in prepared.samples.iter().enumerate() {
            assert_eq!(sample.timestamp_ms, index as i64 * 1_000);
            assert_eq!(sample.window_end_ms, sample.timestamp_ms);
        }
        assert!(prepared.split.train.contains(&9));
        assert!(prepared.split.validation.contains(&19));
        assert!(prepared.split.test.contains(&29));
        assert_eq!(prepared.raw_rows[3]["time"], json!(3_000_999));
        source.temporal_units.clear();
        assert!(prepare_rows(source, &request, &rows).is_err());
        assert_eq!(TemporalUnit::Seconds.to_milliseconds(7), Some(7_000));
        assert_eq!(TemporalUnit::Nanoseconds.to_milliseconds(-1), Some(-1));
        assert_eq!(TemporalUnit::Days.to_milliseconds(2), Some(172_800_000));
        assert_eq!(TemporalUnit::Days.to_milliseconds(i64::MAX), None);
    }

    #[test]
    fn training_profiles_describe_only_mapped_features_and_targets() {
        let (source, request, rows) = fixture();
        let prepared = prepare_rows(source, &request, &rows).unwrap();
        let names: Vec<_> = prepared
            .profile
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect();
        assert_eq!(names, ["category", "target", "x"]);
        assert_eq!(
            mapped_columns(&request.mapping),
            ["id", "group", "time", "x", "category", "target"]
        );
        let x = &prepared.profile.columns[2];
        assert_eq!(x.numeric, prepared.split.train.len());
        assert!(x.examples.len() <= 5);
        assert!(Arc::ptr_eq(&prepared.rows, &prepared.raw_rows));
    }

    fn feature_plan() -> super::super::feature_engineering::FeaturePlan {
        flow_like_types::json::from_value(json!({
            "pipeline":{"steps":[
                {"kind":"derive_columns","columns":[{"name":"x_squared","expression":{"kind":"binary","op":"multiply","left":{"kind":"column","name":"x"},"right":{"kind":"column","name":"x"}}}]},
                {"kind":"window_features","group_by":["group"],"timestamp_column":"time","features":[{"name":"lag_x","column":"x","operation":{"kind":"lag","steps":1}}]},
                {"kind":"join_sources","alias":"calibration","keys":[{"left":"category","right":"category"}],"join":{"kind":"exact"},"columns":[{"source":"offset","name":"calibration_offset"}]}
            ]},
            "numeric_columns":["x","x_squared","lag_x","calibration_offset"],"categorical_columns":[],
            "standardize":false,"imputation":{"kind":"constant","value":0.0}
        })).unwrap()
    }

    #[test]
    fn engineered_features_preserve_names_splits_and_replay_window_state() {
        use super::super::feature_engineering::{PinnedFeatureSource, prepare_feature_rows};
        let (source, request, rows) = fixture();
        let side = vec![PinnedFeatureSource {
            alias: "calibration".into(),
            source: source.clone(),
            allowed_columns: vec!["category".into(), "offset".into()],
        }];
        let side_rows = BTreeMap::from([(
            "calibration".into(),
            vec![json!({"category":"shared","offset":2.5})],
        )]);
        let original = prepare_rows(source.clone(), &request, &rows).unwrap();
        let prepared =
            prepare_feature_rows(source, &request, &rows, &feature_plan(), side, &side_rows)
                .unwrap();
        assert_eq!(prepared.split, original.split);
        assert_eq!(prepared.raw_content_digests, original.raw_content_digests);
        assert_eq!(*prepared.raw_rows, rows);
        for index in prepared
            .split
            .train
            .iter()
            .chain(&prepared.split.validation)
            .chain(&prepared.split.test)
        {
            assert_eq!(prepared.rows[*index]["x"], rows[*index]["x"]);
            assert_eq!(
                prepared.rows[*index]["x_squared"].as_f64().unwrap(),
                (*index * *index) as f64
            );
            assert_eq!(prepared.rows[*index]["calibration_offset"], json!(2.5));
            assert_eq!(
                prepared.samples[*index].annotation,
                original.samples[*index].annotation
            );
        }
        let batch = transform_table_rows(&prepared.preprocessing, &rows[..2], 100).unwrap();
        let (first, state) =
            transform_table_rows_with_state(&prepared.preprocessing, &rows[..1], 100, None, true)
                .unwrap();
        let (second, _) =
            transform_table_rows_with_state(&prepared.preprocessing, &rows[1..2], 100, state, true)
                .unwrap();
        assert_eq!(batch, first.into_iter().chain(second).collect::<Vec<_>>());
        let reloaded: TablePreprocessing =
            flow_like_types::json::from_value(json!(prepared.preprocessing)).unwrap();
        assert_eq!(
            batch,
            transform_table_rows(&reloaded, &rows[..2], 100).unwrap()
        );
    }

    #[test]
    fn feature_plans_reject_undeclared_columns_in_selections_and_steps() {
        use super::super::feature_engineering::{FeaturePlan, prepare_feature_rows};
        let (source, request, rows) = fixture();
        let attempt = |plan: Value| {
            let plan: FeaturePlan = flow_like_types::json::from_value(plan).unwrap();
            prepare_feature_rows(
                source.clone(),
                &request,
                &rows,
                &plan,
                vec![],
                &BTreeMap::new(),
            )
            .unwrap_err()
            .to_string()
        };
        let selected = "Selected features must be authorized inputs or generated columns";
        for (numeric, categorical) in [(json!(["x", "z"]), json!([])), (json!(["x"]), json!(["z"]))]
        {
            let error = attempt(json!({
                "numeric_columns":numeric,"categorical_columns":categorical,"standardize":true
            }));
            assert!(error.contains(selected), "{error}");
        }
        let steps = [
            (
                json!({"kind":"derive_columns","columns":[{"name":"z_double","expression":{"kind":"binary","op":"add","left":{"kind":"column","name":"z"},"right":{"kind":"column","name":"z"}}}]}),
                "Column 'z' is not an allowed feature input",
            ),
            (
                json!({"kind":"window_features","group_by":["group"],"timestamp_column":"time","features":[{"name":"lag_z","column":"z","operation":{"kind":"lag","steps":1}}]}),
                "Column 'z' is not an allowed feature input",
            ),
            (
                json!({"kind":"window_features","group_by":["z"],"timestamp_column":"time","features":[{"name":"lag_x","column":"x","operation":{"kind":"lag","steps":1}}]}),
                "Window key 'z' is not allowed metadata",
            ),
        ];
        for (step, expected) in steps {
            let error = attempt(json!({
                "pipeline":{"steps":[step]},
                "numeric_columns":["x"],"categorical_columns":[],"standardize":true
            }));
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn window_features_match_a_full_time_ordered_replay_in_every_partition() {
        use super::super::feature_engineering::prepare_feature_rows;
        let (source, request, mut rows) = fixture();
        rows.reverse();
        let plan: super::super::feature_engineering::FeaturePlan =
            flow_like_types::json::from_value(json!({
                "pipeline":{"steps":[
                    {"kind":"window_features","group_by":["category"],"timestamp_column":"time","features":[
                        {"name":"lag_x","column":"x","operation":{"kind":"lag","steps":1}},
                        {"name":"mean_x","column":"x","operation":{"kind":"rolling","statistic":"mean","window":{"kind":"rows","count":3},"min_periods":1,"include_current":false}}
                    ]}
                ]},
                "numeric_columns":["x","lag_x","mean_x"],"categorical_columns":[],
                "standardize":false,"imputation":{"kind":"constant","value":0.0}
            }))
            .unwrap();
        let prepared =
            prepare_feature_rows(source, &request, &rows, &plan, vec![], &BTreeMap::new()).unwrap();
        let TablePreprocessing::Engineered { pipeline, .. } = &prepared.preprocessing else {
            unreachable!()
        };
        assert_eq!(
            pipeline.limits.maximum_duration_ms,
            plan.limits.maximum_duration_ms
        );
        assert_eq!(*prepared.raw_rows, rows);
        let groups: HashSet<_> = prepared
            .split
            .train
            .iter()
            .map(|index| &prepared.samples[*index].group_id)
            .collect();
        assert!(
            prepared
                .split
                .validation
                .iter()
                .chain(&prepared.split.test)
                .all(|index| !groups.contains(&prepared.samples[*index].group_id))
        );
        let mut ordered: Vec<usize> = (0..rows.len()).collect();
        ordered.sort_by_key(|index| rows[*index]["time"].as_i64().unwrap());
        let ordered_rows: Vec<Value> = ordered.iter().map(|index| rows[*index].clone()).collect();
        let replay = transform_table_rows(&prepared.preprocessing, &ordered_rows, 1_000).unwrap();
        for (position, index) in ordered.iter().enumerate() {
            let time = rows[*index]["time"].as_i64().unwrap() as f32;
            let lag = if time > 0. { time - 1. } else { 0. };
            let mean = match time as i64 {
                0 => 0.,
                1 => 0.,
                2 => 0.5,
                _ => time - 2.,
            };
            let expected = vec![time, lag, mean];
            assert_eq!(replay[position].values, expected);
            let partitioned = prepared.split.train.contains(index)
                || prepared.split.validation.contains(index)
                || prepared.split.test.contains(index);
            if partitioned {
                assert_eq!(prepared.samples[*index].input, replay[position]);
            }
        }
    }

    #[test]
    fn engineered_fit_never_uses_held_out_values_and_blocks_target_expressions() {
        use super::super::feature_engineering::prepare_feature_rows;
        let (source, request, mut rows) = fixture();
        let mut plan = feature_plan();
        plan.pipeline.steps.truncate(1);
        plan.numeric_columns = vec!["x", "x_squared"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        plan.standardize = true;
        plan.pca_components = Some(1);
        let original = prepare_feature_rows(
            source.clone(),
            &request,
            &rows,
            &plan,
            vec![],
            &BTreeMap::new(),
        )
        .unwrap();
        for index in original.split.validation.iter().chain(&original.split.test) {
            rows[*index]["x"] = json!(1_000_000);
        }
        let changed = prepare_feature_rows(
            source.clone(),
            &request,
            &rows,
            &plan,
            vec![],
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(json!(original.preprocessing), json!(changed.preprocessing));
        assert_eq!(original.spec.input_shape, vec![1]);
        let index = original.split.train[0];
        assert!(original.rows[index].get("__ml_pca_1").is_some());
        let replay = transform_table_rows(
            &original.preprocessing,
            &original.raw_rows[index..index + 1],
            32,
        )
        .unwrap();
        assert_eq!(replay[0], original.samples[index].input);
        plan.limits.maximum_columns = original.rows[index].as_object().unwrap().len() - 1;
        assert!(
            prepare_feature_rows(
                source.clone(),
                &request,
                &rows,
                &plan,
                vec![],
                &BTreeMap::new(),
            )
            .unwrap_err()
            .to_string()
            .contains("column budget")
        );
        plan.limits = Default::default();
        let bad=flow_like_types::json::from_value(json!({"steps":[{"kind":"derive_columns","columns":[{"name":"leak","expression":{"kind":"column","name":"target"}}]}]})).unwrap();
        plan.pipeline = bad;
        plan.numeric_columns = vec!["leak".into()];
        assert!(
            prepare_feature_rows(source, &request, &rows, &plan, vec![], &BTreeMap::new()).is_err()
        );
    }

    #[test]
    fn learning_selection_and_review_overlays_preserve_raw_identity() {
        let (source, mut request, rows) = fixture();
        let original = prepare_rows(source.clone(), &request, &rows).unwrap();
        let audit = original.split.test[0];
        request
            .selection
            .excluded_groups
            .push(original.samples[audit].group_id.clone());
        let trained = original.split.test[2];
        request
            .selection
            .ineligible_test_ids
            .push(original.samples[trained].id.clone());
        let reviewed = original.split.train[0];
        request.label_overrides.insert(
            original.samples[reviewed].id.clone(),
            TableLabelOverride {
                annotation: Annotation::Class { class_id: 1 },
                provenance: LabelProvenance::Reviewed {
                    reviewer: "quality".into(),
                    reviewed_at_ms: 100,
                },
                annotation_revision: 2,
                outcome: None,
                available_at_ms: Some(100),
            },
        );
        let changed = prepare_rows(source, &request, &rows).unwrap();
        assert!(
            !changed
                .split
                .train
                .iter()
                .chain(&changed.split.validation)
                .chain(&changed.split.test)
                .any(|index| changed.samples[*index].group_id == original.samples[audit].group_id)
        );
        assert!(!changed.split.test.contains(&trained));
        assert_eq!(
            changed.samples[reviewed].annotation,
            Annotation::Class { class_id: 1 }
        );
        assert_eq!(changed.raw_content_digests, original.raw_content_digests);
        assert_eq!(*changed.rows, rows);
        changed.split.validate(&changed.samples).unwrap();
    }

    #[test]
    fn engineered_datasets_keep_excluded_rows_out_of_fitting_and_materialization() {
        let (source, mut request, rows) = fixture();
        let original = prepare_rows(source.clone(), &request, &rows).unwrap();
        let excluded = original.split.test[0];
        request.selection.excluded_groups = vec![original.samples[excluded].group_id.clone()];
        let mut plan = feature_plan();
        plan.pipeline.steps.truncate(1);
        plan.numeric_columns = vec!["x_squared".into()];
        let prepared = super::super::feature_engineering::prepare_feature_rows(
            source.clone(),
            &request,
            &rows,
            &plan,
            vec![],
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(prepared.split.purged.contains(&excluded));
        assert!(
            !prepared
                .split
                .train
                .iter()
                .chain(&prepared.split.validation)
                .chain(&prepared.split.test)
                .any(|index| *index == excluded)
        );
        assert_eq!(
            prepared.rows[excluded]["x_squared"].as_f64(),
            Some((excluded * excluded) as f64)
        );
        assert_eq!(prepared.raw_rows[excluded], rows[excluded]);
        prepared.split.validate(&prepared.samples).unwrap();
        plan.limits.maximum_rows = rows.len() - 1;
        assert!(
            super::super::feature_engineering::prepare_feature_rows(
                source,
                &request,
                &rows,
                &plan,
                vec![],
                &BTreeMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn stateful_replay_bounds_the_whole_transformed_batch() {
        let (source, request, rows) = fixture();
        let mut plan = feature_plan();
        plan.pipeline.steps.truncate(1);
        plan.numeric_columns = vec!["x_squared".into()];
        let mut prepared = super::super::feature_engineering::prepare_feature_rows(
            source,
            &request,
            &rows,
            &plan,
            vec![],
            &BTreeMap::new(),
        )
        .unwrap();
        let TablePreprocessing::Engineered { pipeline, .. } = &mut prepared.preprocessing else {
            unreachable!()
        };
        pipeline.limits.maximum_bytes = rows
            .iter()
            .map(|row| serialized_json_size(row).unwrap())
            .sum();
        let error =
            transform_table_rows_with_state(&prepared.preprocessing, &rows, 1024, None, true)
                .unwrap_err();
        assert!(error.to_string().contains("Transformed feature batch"));
    }

    #[cfg(feature = "execute")]
    struct TestDirectory(std::path::PathBuf);
    #[cfg(feature = "execute")]
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A context whose project database and app storage live in a fresh temporary directory.
    #[cfg(feature = "execute")]
    async fn table_context() -> (
        TestDirectory,
        flow_like::flow::execution::context::ExecutionContext,
        flow_like_storage::lancedb::Connection,
    ) {
        use ahash::AHashMap;
        use flow_like::{
            flow::{
                board::ExecutionStage,
                execution::{
                    LogLevel, Run,
                    context::{ExecutionContext, ExecutionContextCache},
                    internal_node::InternalNode,
                },
                node::NodeLogic,
            },
            profile::Profile,
            state::{FlowLikeConfig, FlowLikeState, FlowLikeStores},
            utils::http::HTTPClient,
        };
        use flow_like_storage::{
            files::store::{FlowLikeStore, local_store::LocalObjectStore},
            object_store::path::Path,
        };
        use flow_like_types::sync::{Mutex, RwLock};
        use std::sync::Weak;
        let directory = TestDirectory(std::env::temp_dir().join(format!(
            "flow-like-training-tables-{}",
            uuid::Uuid::new_v4()
        )));
        std::fs::create_dir_all(&directory.0).unwrap();
        let location = directory.0.join("database").to_string_lossy().into_owned();
        let mut config = FlowLikeConfig::new();
        let reopen_location = location.clone();
        config.register_build_project_database(Arc::new(move |_| {
            flow_like_storage::lancedb::connect(&reopen_location)
        }));
        let state = Arc::new(FlowLikeState::new(
            config,
            HTTPClient::new_without_refetch(),
        ));
        let logic: Arc<dyn NodeLogic> = Arc::new(super::super::data::MakeInspectionSampleNode);
        let node = Arc::new(InternalNode::new(
            logic.get_node(),
            AHashMap::new(),
            logic,
            AHashMap::new(),
        ));
        let mut nodes = AHashMap::new();
        nodes.insert(node.node_id().to_string(), node.clone());
        let run: Weak<Mutex<Run>> = Weak::new();
        let mut context = ExecutionContext::new(
            Arc::new(nodes),
            &run,
            &state,
            &node,
            &Arc::new(Mutex::new(AHashMap::new())),
            &Arc::new(RwLock::new(AHashMap::new())),
            LogLevel::Debug,
            ExecutionStage::Dev,
            Arc::new(Profile::default()),
            None,
            Arc::new(RwLock::new(Vec::new())),
            None,
            None,
            Arc::new(AHashMap::new()),
            None,
        )
        .await;
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = Some(FlowLikeStore::Local(Arc::new(
            LocalObjectStore::new(directory.0.clone()).unwrap(),
        )));
        context.execution_cache = Some(ExecutionContextCache {
            stores,
            app_id: "table-test".into(),
            model_usage_app_id: None,
            board_dir: Path::from("board"),
            board_id: "board".into(),
            node_id: node.shared_node_id(),
            sub: "tester".into(),
            shadow: false,
        });
        let connection = flow_like_storage::lancedb::connect(&location)
            .execute()
            .await
            .unwrap();
        (directory, context, connection)
    }

    /// Writes `rows` to a new table and caches it as an unpinned run database.
    #[cfg(feature = "execute")]
    async fn cached_table(
        context: &mut flow_like::flow::execution::context::ExecutionContext,
        connection: flow_like_storage::lancedb::Connection,
        name: &str,
        rows: Vec<Value>,
    ) -> NodeDBConnection {
        use flow_like_catalog_core::CachedDB;
        use flow_like_storage::databases::vector::{
            VectorStore, buffered::BufferedVectorStore, lancedb::LanceDBVectorStore,
        };
        use flow_like_types::sync::RwLock;
        let mut store = LanceDBVectorStore::from_connection(connection, name.into()).await;
        store.insert(rows).await.unwrap();
        let database = NodeDBConnection {
            cache_key: format!("source::{name}"),
        };
        context.cache.write().await.insert(
            database.cache_key.clone(),
            Arc::new(CachedDB {
                db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
            }),
        );
        database
    }

    #[cfg(feature = "execute")]
    #[tokio::test]
    async fn local_table_pin_materialization_retry_and_reopen_preserve_source_versions() {
        use flow_like_storage::databases::vector::VectorStore;
        let (_directory, mut context, connection) = table_context().await;
        let (_, mut request, rows) = fixture();
        let database = cached_table(&mut context, connection, "original", rows.clone()).await;
        let initial = database
            .load(&mut context)
            .await
            .unwrap()
            .db
            .read()
            .await
            .inner()
            .reference()
            .await
            .unwrap();
        request.source = database.clone();
        let prepared = prepare_table_dataset(&mut context, &request).await.unwrap();
        assert!(prepared.raw_rows.iter().all(|row| row.get("z").is_none()));
        assert_eq!(prepared.profile.columns.len(), 3);
        let callback = context
            .app_state
            .config
            .write()
            .await
            .callbacks
            .build_project_database
            .take();
        let cached_pin = reopen_pinned_source(&mut context, &prepared.source)
            .await
            .unwrap();
        assert_eq!(cached_pin.cache_key, prepared.source.database.cache_key);
        let mut mismatched = prepared.source.clone();
        mismatched.reference.version += 1;
        assert!(
            reopen_pinned_source(&mut context, &mismatched)
                .await
                .is_err()
        );
        context
            .app_state
            .config
            .write()
            .await
            .callbacks
            .build_project_database = callback;
        let cloned = clone_experiment_table(&mut context, &prepared.source, "independent_clone")
            .await
            .unwrap();
        assert!(!cloned.reference.read_only);
        assert_eq!(
            cloned
                .database
                .load(&mut context)
                .await
                .unwrap()
                .db
                .read()
                .await
                .filter("true", None, 100, 0)
                .await
                .unwrap()
                .len(),
            rows.len()
        );
        let policy = TableWritePolicy::default();
        let materialized =
            materialize_prepared_tables(&mut context, &prepared, "experiment-1", &policy)
                .await
                .unwrap();
        assert_eq!(materialized.len(), 3);
        for table in &materialized {
            assert!(table.reference.pinned && table.reference.read_only);
            assert_ne!(table.reference.table, initial.table);
        }
        let retried = materialize_prepared_tables(&mut context, &prepared, "experiment-1", &policy)
            .await
            .unwrap();
        assert_eq!(
            materialized
                .iter()
                .map(|table| &table.reference)
                .collect::<Vec<_>>(),
            retried
                .iter()
                .map(|table| &table.reference)
                .collect::<Vec<_>>()
        );
        assert!(write_experiment_rows(&mut context,&prepared.source,vec![json!({"sample_id":prepared.samples[prepared.split.train[0]].id,"unexpected":"change"})],"experiment-1","train","different",&policy).await.is_err());
        let mut plan = feature_plan();
        plan.pipeline.steps.truncate(1);
        plan.numeric_columns = vec!["x".into(), "x_squared".into()];
        let engineered = super::super::feature_engineering::prepare_feature_rows(
            prepared.source.clone(),
            &request,
            &rows,
            &plan,
            vec![],
            &BTreeMap::new(),
        )
        .unwrap();
        let engineered_tables =
            materialize_prepared_tables(&mut context, &engineered, "experiment-features", &policy)
                .await
                .unwrap();
        for table in engineered_tables {
            let stored = table.database.load(&mut context).await.unwrap();
            let stored_rows = stored
                .db
                .read()
                .await
                .filter("true", None, 100, 0)
                .await
                .unwrap();
            for row in stored_rows {
                let x = row["x"].as_f64().unwrap();
                assert_eq!(row["x_squared"].as_f64().unwrap(), x * x);
                assert!(row.get("target").is_some());
                assert!(row.get("category").is_some());
            }
        }
        let cached = database.load(&mut context).await.unwrap();
        assert_eq!(
            cached.db.read().await.inner().reference().await.unwrap(),
            initial
        );
        let mut changed = rows[0].clone();
        changed["x"] = json!(9999);
        cached
            .upsert_from(&context, vec![changed], "id".into())
            .await
            .unwrap();
        cached.ensure_flushed().await.unwrap();
        assert!(
            cached
                .db
                .read()
                .await
                .inner()
                .reference()
                .await
                .unwrap()
                .version
                > initial.version
        );
        context.cache.write().await.clear();
        let reopened = reopen_pinned_source(&mut context, &prepared.source)
            .await
            .unwrap();
        request.source = reopened;
        let again = prepare_table_dataset(&mut context, &request).await.unwrap();
        assert_eq!(again.samples, prepared.samples);
        assert_eq!(again.digest, prepared.digest);
        request.source = reopen_latest_source(&mut context, &prepared.source)
            .await
            .unwrap();
        let latest = prepare_table_dataset(&mut context, &request).await.unwrap();
        assert!(latest.source.reference.version > prepared.source.reference.version);
        assert!(
            latest
                .raw_rows
                .iter()
                .any(|row| row["id"] == 0 && row["x"] == 9999)
        );
        context.execution_cache.as_mut().unwrap().shadow = true;
        assert!(
            materialize_prepared_tables(&mut context, &again, "shadow", &policy)
                .await
                .is_err()
        );
    }

    #[cfg(feature = "execute")]
    #[tokio::test]
    async fn latest_reopen_survives_a_cleaned_pinned_version_that_exact_replays_name() {
        use flow_like_storage::lancedb::table::{Duration, OptimizeAction};
        let (_directory, mut context, connection) = table_context().await;
        let (_, mut request, rows) = fixture();
        let database = cached_table(&mut context, connection, "cleaned", rows.clone()).await;
        request.source = database.clone();
        let prepared = prepare_table_dataset(&mut context, &request).await.unwrap();
        let cached = database.load(&mut context).await.unwrap();
        let mut changed = rows[1].clone();
        changed["x"] = json!(4242);
        cached
            .upsert_from(&context, vec![changed], "id".into())
            .await
            .unwrap();
        cached.ensure_flushed().await.unwrap();
        let table = cached.db.read().await.inner().raw().await.unwrap();
        table
            .optimize(OptimizeAction::Prune {
                older_than: Some(Duration::zero()),
                delete_unverified: Some(true),
                error_if_tagged_old_versions: Some(false),
            })
            .await
            .unwrap();
        context.cache.write().await.clear();
        let error = reopen_pinned_source(&mut context, &prepared.source)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("was cleaned up"), "{error}");
        request.source = reopen_latest_source(&mut context, &prepared.source)
            .await
            .unwrap();
        let latest = prepare_table_dataset(&mut context, &request).await.unwrap();
        assert!(latest.source.reference.version > prepared.source.reference.version);
        assert!(
            latest
                .raw_rows
                .iter()
                .any(|row| row["id"] == 1 && row["x"] == 4242)
        );
    }

    #[cfg(feature = "execute")]
    #[tokio::test]
    async fn mapped_reads_project_columns_in_byte_sized_pages_within_budgets() {
        let (_directory, mut context, connection) = table_context().await;
        let (_, request, rows) = fixture();
        let database = cached_table(&mut context, connection, "paged", rows.clone()).await;
        let source = pin_table_source(&mut context, &database).await.unwrap();
        let columns = mapped_columns(&request.mapping);
        let mut budget = TableBudget {
            maximum_bytes: 4096,
            ..TableBudget::default()
        };
        let read = read_columns(&mut context, &source, &budget, &columns)
            .await
            .unwrap();
        assert_eq!(read.len(), rows.len());
        for (row, original) in read.iter().zip(&rows) {
            let expected = Value::Object(
                columns
                    .iter()
                    .map(|name| (name.clone(), original[name].clone()))
                    .collect(),
            );
            assert_eq!(row, &expected);
        }
        budget.maximum_rows = rows.len() - 1;
        let error = read_columns(&mut context, &source, &budget, &columns)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("row budget"), "{error}");
        budget.maximum_rows = rows.len();
        budget.maximum_bytes = 1024;
        assert!(
            read_columns(&mut context, &source, &budget, &columns)
                .await
                .is_err()
        );
        let whole = read_rows(&mut context, &source, &TableBudget::default(), 7)
            .await
            .unwrap();
        assert_eq!(whole, rows);
    }

    #[cfg(feature = "execute")]
    #[tokio::test]
    async fn concurrent_materialization_creates_each_experiment_table_once() {
        use flow_like_storage::databases::vector::{VectorStore, lancedb::LanceDBVectorStore};
        let (_directory, mut context, connection) = table_context().await;
        let (_, _, rows) = fixture();
        let database = cached_table(&mut context, connection.clone(), "raced", rows).await;
        let source = pin_table_source(&mut context, &database).await.unwrap();
        let output: Vec<Value> = (0..5)
            .map(|index| json!({"sample_id":format!("s{index}"),"record_json":"{}"}))
            .collect();
        let policy = TableWritePolicy::default();
        let mut other_context = context.clone();
        let (first, second) = tokio::join!(
            write_experiment_rows(
                &mut context,
                &source,
                output.clone(),
                "experiment-race",
                "predictions",
                "digest",
                &policy,
            ),
            write_experiment_rows(
                &mut other_context,
                &source,
                output.clone(),
                "experiment-race",
                "predictions",
                "digest",
                &policy,
            )
        );
        let (first, second) = (first.unwrap(), second.unwrap());
        assert_eq!(first.reference, second.reference);
        let name = first.reference.table.clone();
        assert!(
            !table_io::create_experiment_table(&connection, &name, &output)
                .await
                .unwrap()
        );
        let stored = LanceDBVectorStore::from_connection(connection, name).await;
        assert_eq!(stored.count(None).await.unwrap(), output.len());
    }

    #[cfg(feature = "execute")]
    #[tokio::test]
    async fn pinned_sources_record_non_millisecond_temporal_columns() {
        use flow_like_catalog_core::CachedDB;
        use flow_like_storage::{
            arrow_array::{
                ArrayRef, Date32Array, Int64Array, RecordBatch, StringArray,
                TimestampMicrosecondArray,
            },
            arrow_schema::{DataType, Field, Schema, TimeUnit},
            databases::vector::{buffered::BufferedVectorStore, lancedb::LanceDBVectorStore},
        };
        use flow_like_types::sync::RwLock;
        let (_directory, mut context, connection) = table_context().await;
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new(
                "captured",
                DataType::Timestamp(TimeUnit::Microsecond, None),
                false,
            ),
            Field::new("day", DataType::Date32, false),
            Field::new("label", DataType::Utf8, false),
        ]));
        let columns: Vec<ArrayRef> = vec![
            Arc::new(Int64Array::from(vec![1, 2])),
            Arc::new(TimestampMicrosecondArray::from(vec![1_500_000, 2_500_000])),
            Arc::new(Date32Array::from(vec![1, 2])),
            Arc::new(StringArray::from(vec!["a", "b"])),
        ];
        let batch = RecordBatch::try_new(schema, columns).unwrap();
        let mut store = LanceDBVectorStore::from_connection(connection, "timed".into()).await;
        store.insert_record_batch(batch).await.unwrap();
        let database = NodeDBConnection {
            cache_key: "timed".into(),
        };
        context.cache.write().await.insert(
            database.cache_key.clone(),
            Arc::new(CachedDB {
                db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
            }),
        );
        let source = pin_table_source(&mut context, &database).await.unwrap();
        assert_eq!(
            source.temporal_units,
            BTreeMap::from([
                ("captured".to_string(), TemporalUnit::Microseconds),
                ("day".to_string(), TemporalUnit::Days),
            ])
        );
        let rows = read_rows(&mut context, &source, &TableBudget::default(), 10)
            .await
            .unwrap();
        assert_eq!(rows[0]["captured"], json!(1_500_000));
        assert_eq!(rows[1]["day"], json!(2));
    }

    /// An auto-training request over the fixture columns; `dataset` adds stream and split.
    #[cfg(feature = "training")]
    fn training_request(
        source: &NodeDBConnection,
        dataset: Value,
    ) -> super::super::auto_training::AutoTrainRequest {
        let mut request = json!({
            "dataset":{
                "source":source,
                "spec":{"id":"readings","description":"Classify readings","task":"sensor_classification","labels":["good","bad"],"input_shape":[1],"prediction_horizon_ms":null,"minimum_examples":2,"minimum_examples_per_class":1},
                "mapping":{"row_id":"id","group_id":"group","timestamp_ms":"time","features":{"kind":"tabular","options":{"numeric_columns":["x"],"categorical_columns":["category"]}},"target":{"kind":"column","column":"target"},"provenance":{"kind":"measured","source":"gauge"}}
            },
            "compute":{"backend":"cpu"},
            "tables":{"create_tables":false}
        });
        for (key, value) in dataset.as_object().unwrap() {
            request["dataset"][key] = value.clone();
        }
        flow_like_types::json::from_value(request).unwrap()
    }

    #[cfg(feature = "training")]
    #[tokio::test]
    async fn temporal_boundary_samples_import_into_their_partitions() {
        use super::super::auto_training::{experiment_repository, start_auto_training};
        let (_directory, mut context, connection) = table_context().await;
        let rows: Vec<Value> = (0..=30)
            .map(|index| {
                json!({"id":index,"group":index,"time":index,"x":index,"category":"shared",
                    "target":if index % 2 == 0 {"good"} else {"bad"}})
            })
            .collect();
        let source = cached_table(&mut context, connection, "boundaries", rows).await;
        for embargo in [0i64, 2] {
            let request = training_request(
                &source,
                json!({
                    "stream_id":format!("boundaries-{embargo}"),
                    "split":{"kind":"time","train_end_ms":10,"validation_end_ms":20,"embargo_ms":embargo}
                }),
            );
            let started = start_auto_training(&mut context, request, false)
                .await
                .unwrap();
            let repo = experiment_repository(&context, &started.experiment_id).unwrap();
            let experiment = repo.get_experiment(&started.experiment_id).unwrap();
            let snapshot = repo
                .get_snapshot_limited(
                    &experiment.request.snapshot_id,
                    experiment.request.budget.maximum_dataset_bytes,
                )
                .unwrap();
            let ids = |samples: &[flow_like_ml_runtime::TrainingSample]| {
                samples
                    .iter()
                    .map(|sample| sample.id.clone())
                    .collect::<HashSet<_>>()
            };
            let train = ids(&snapshot.train);
            let validation = ids(&snapshot.validation);
            let test = ids(&snapshot.test);
            assert!(train.contains("10"));
            assert!(validation.contains(&(11 + embargo).to_string()));
            assert!(validation.contains("20"));
            assert!(test.contains(&(21 + embargo).to_string()));
            for offset in 1..=embargo {
                for id in [(10 + offset).to_string(), (20 + offset).to_string()] {
                    assert!(
                        !train.contains(&id) && !validation.contains(&id) && !test.contains(&id)
                    );
                }
            }
        }
    }

    #[cfg(feature = "training")]
    #[tokio::test]
    async fn feature_candidates_reject_undeclared_source_columns() {
        use super::super::feature_engineering::{FeaturePlan, derive_pipeline_candidate};
        let (_directory, mut context, connection) = table_context().await;
        let (_, _, rows) = fixture();
        let source = cached_table(&mut context, connection, "undeclared", rows).await;
        let mut request = training_request(
            &source,
            json!({
                "stream_id":"undeclared",
                "split":{"kind":"group","train_fraction":0.6,"validation_fraction":0.2,"seed":42}
            }),
        );
        request.feature_plan = Some(
            flow_like_types::json::from_value(json!({
                "pipeline":{"steps":[{"kind":"derive_columns","columns":[{"name":"x_squared","expression":{"kind":"unary","op":"square","value":{"kind":"column","name":"x"}}}]}]},
                "numeric_columns":["x","x_squared"],"categorical_columns":[],"standardize":true
            }))
            .unwrap(),
        );
        let id = super::super::auto_training::start_auto_training(&mut context, request, false)
            .await
            .unwrap()
            .experiment_id;
        for (numeric, categorical) in [(vec!["z"], vec![]), (vec!["x"], vec!["z"])] {
            assert!(
                super::super::auto_training::derive_feature_candidate(
                    &mut context,
                    &id,
                    numeric.into_iter().map(String::from).collect(),
                    categorical.into_iter().map(String::from).collect(),
                    true,
                )
                .await
                .is_err()
            );
        }
        let plans = [
            (
                json!({"numeric_columns":["x","z"],"categorical_columns":[],"standardize":true}),
                "Selected features must be authorized inputs or generated columns",
            ),
            (
                json!({"pipeline":{"steps":[{"kind":"derive_columns","columns":[{"name":"z_double","expression":{"kind":"binary","op":"add","left":{"kind":"column","name":"z"},"right":{"kind":"column","name":"z"}}}]}]},"numeric_columns":["x","z_double"],"categorical_columns":[],"standardize":true}),
                "Column 'z' is not an allowed feature input",
            ),
            (
                json!({"pipeline":{"steps":[{"kind":"window_features","group_by":["z"],"timestamp_column":"time","features":[{"name":"lag_x","column":"x","operation":{"kind":"lag","steps":1}}]}]},"numeric_columns":["x","lag_x"],"categorical_columns":[],"standardize":true}),
                "Window key 'z' is not allowed metadata",
            ),
        ];
        for (plan, expected) in plans {
            let plan: FeaturePlan = flow_like_types::json::from_value(plan).unwrap();
            let error = derive_pipeline_candidate(&mut context, &id, plan)
                .await
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
}
