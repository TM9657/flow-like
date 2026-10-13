use super::auto_training_tables::{
    FeatureSource, MaterializedTable, PinnedTableSource, PreparedTableDataset, TableDatasetRequest,
    TablePreprocessing, TableWritePolicy,
};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
};
use flow_like_catalog_core::NodeDBConnection;
use flow_like_ml_native::{
    feature_engineering::{FeatureLimits, FeaturePipeline, FeaturePolicy, FeatureStep},
    preprocessing::Imputation,
};
use flow_like_types::{Result, Value, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::Arc,
    time::Instant,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct FeaturePlan {
    #[serde(default)]
    pub pipeline: FeaturePipeline,
    pub numeric_columns: Vec<String>,
    pub categorical_columns: Vec<String>,
    #[serde(default)]
    pub imputation: Imputation,
    pub standardize: bool,
    #[serde(default)]
    pub pca_components: Option<usize>,
    #[serde(default)]
    pub limits: FeatureLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct FeatureSourceInput {
    pub alias: String,
    pub source: NodeDBConnection,
    pub allowed_columns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PinnedFeatureSource {
    pub alias: String,
    pub source: PinnedTableSource,
    pub allowed_columns: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct FeatureTransformRequest {
    pub dataset: TableDatasetRequest,
    pub plan: FeaturePlan,
    #[serde(default)]
    pub sources: Vec<FeatureSourceInput>,
    pub output_id: String,
    #[serde(default)]
    pub tables: TableWritePolicy,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct FeatureTransformResult {
    pub prepared: PreparedTableDataset,
    pub tables: Vec<MaterializedTable>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct FitPreprocessingRequest {
    pub dataset: TableDatasetRequest,
    pub plan: FeaturePlan,
    #[serde(default)]
    pub sources: Vec<FeatureSourceInput>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaterializeDatasetRequest {
    pub prepared: PreparedTableDataset,
    pub output_id: String,
    #[serde(default)]
    pub tables: TableWritePolicy,
}

fn policy(request: &TableDatasetRequest, sources: &[PinnedFeatureSource]) -> Result<FeaturePolicy> {
    let FeatureSource::Tabular { options } = &request.mapping.features else {
        return Err(anyhow!(
            "Feature engineering requires a tabular source mapping"
        ));
    };
    let mut forbidden = vec![request.mapping.row_id.clone()];
    if let Some(value) = &request.mapping.outcome {
        forbidden.push(value.clone());
    }
    match &request.mapping.target {
        super::auto_training_tables::TargetSource::Column { column }
        | super::auto_training_tables::TargetSource::Annotation { column } => {
            forbidden.push(column.clone())
        }
        super::auto_training_tables::TargetSource::Columns { columns } => {
            forbidden.extend(columns.clone())
        }
        super::auto_training_tables::TargetSource::Sample => {
            return Err(anyhow!(
                "Feature engineering needs an explicit target mapping"
            ));
        }
    }
    let mut metadata = vec![request.mapping.row_id.clone()];
    metadata.extend(
        [
            &request.mapping.group_id,
            &request.mapping.timestamp_ms,
            &request.mapping.window_start_ms,
            &request.mapping.window_end_ms,
        ]
        .into_iter()
        .flatten()
        .cloned(),
    );
    let mut aliases = BTreeMap::new();
    for source in sources {
        if source.alias.trim().is_empty()
            || source.alias.len() > 128
            || aliases
                .insert(source.alias.clone(), source.allowed_columns.clone())
                .is_some()
        {
            return Err(anyhow!("Feature sources need unique bounded aliases"));
        }
    }
    Ok(FeaturePolicy {
        allowed_input_columns: options
            .numeric_columns
            .iter()
            .chain(&options.categorical_columns)
            .cloned()
            .collect(),
        forbidden_columns: forbidden,
        metadata_columns: metadata,
        allowed_sources: aliases,
    })
}

/// Upper bounds of the native window state limits.
const MAXIMUM_WINDOW_GROUPS: usize = 100_000;
const MAXIMUM_WINDOW_HISTORY_ROWS: usize = 1_000_000;

/// One duration budget for fitting a feature plan and transforming every row with it.
struct PreparationDeadline {
    started: Instant,
    budget_ms: u64,
}
impl PreparationDeadline {
    fn remaining_ms(&self) -> Result<u64> {
        let elapsed = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        match self.budget_ms.checked_sub(elapsed) {
            Some(remaining) if remaining > 0 => Ok(remaining),
            _ => Err(anyhow!(
                "Feature preparation exceeded its {} ms duration budget",
                self.budget_ms
            )),
        }
    }
}

pub fn prepare_feature_rows(
    source: PinnedTableSource,
    request: &TableDatasetRequest,
    rows: &[Value],
    plan: &FeaturePlan,
    sources: Vec<PinnedFeatureSource>,
    side_rows: &BTreeMap<String, Vec<Value>>,
) -> Result<PreparedTableDataset> {
    prepare_feature_rows_owned(source, request, rows.to_vec(), plan, sources, side_rows)
}

/// `prepare_feature_rows` for rows the caller hands over. Window features read feature
/// values only, so one causal pass over every row in time order gives each partition the
/// history that serving sees; fitted statistics still come from training rows alone.
pub fn prepare_feature_rows_owned(
    source: PinnedTableSource,
    request: &TableDatasetRequest,
    rows: Vec<Value>,
    plan: &FeaturePlan,
    sources: Vec<PinnedFeatureSource>,
    side_rows: &BTreeMap<String, Vec<Value>>,
) -> Result<PreparedTableDataset> {
    prepare_feature_rows_within(
        Instant::now(),
        source,
        request,
        rows,
        plan,
        sources,
        side_rows,
    )
}

/// Side rows whose as-of clock columns are converted to epoch milliseconds, or `None` when no
/// side source stores them in another unit.
fn side_rows_in_milliseconds(
    pipeline: &FeaturePipeline,
    sources: &[PinnedFeatureSource],
    side_rows: &BTreeMap<String, Vec<Value>>,
) -> Result<Option<BTreeMap<String, Vec<Value>>>> {
    let mut converted: Option<BTreeMap<String, Vec<Value>>> = None;
    for source in sources {
        let units = super::auto_training_tables::side_time_units(
            pipeline,
            &source.alias,
            &source.source.temporal_units,
        );
        let Some(rows) = side_rows.get(&source.alias).filter(|_| !units.is_empty()) else {
            continue;
        };
        let rows = rows
            .iter()
            .map(|row| super::auto_training_tables::row_in_milliseconds(row, &units))
            .collect::<Result<Vec<_>>>()?;
        converted
            .get_or_insert_with(|| side_rows.clone())
            .insert(source.alias.clone(), rows);
    }
    Ok(converted)
}

/// The preparation, with its duration budget counted from `started`, so fitting the plan and
/// transforming every row share it.
pub(crate) fn prepare_feature_rows_within(
    started: Instant,
    source: PinnedTableSource,
    request: &TableDatasetRequest,
    rows: Vec<Value>,
    plan: &FeaturePlan,
    sources: Vec<PinnedFeatureSource>,
    side_rows: &BTreeMap<String, Vec<Value>>,
) -> Result<PreparedTableDataset> {
    let policy = policy(request, &sources)?;
    let mut limits = plan.limits.clone();
    limits.maximum_rows = limits.maximum_rows.min(request.budget.maximum_rows);
    limits.maximum_bytes = limits.maximum_bytes.min(request.budget.maximum_bytes);
    let deadline = PreparationDeadline {
        started,
        budget_ms: limits.maximum_duration_ms,
    };
    let converted_sides = side_rows_in_milliseconds(&plan.pipeline, &sources, side_rows)?;
    let mut fitted =
        plan.pipeline
            .fit(converted_sides.as_ref().unwrap_or(side_rows), &policy, &limits)?;
    let generated = plan.pipeline.generated_columns();
    let allowed: HashSet<_> = policy
        .allowed_input_columns
        .iter()
        .chain(&generated)
        .collect();
    if plan
        .numeric_columns
        .iter()
        .chain(&plan.categorical_columns)
        .any(|name| !allowed.contains(name))
    {
        return Err(anyhow!(
            "Selected features must be authorized inputs or generated columns"
        ));
    }
    let mut bounded_request = request.clone();
    bounded_request.budget.maximum_rows = limits.maximum_rows;
    bounded_request.budget.maximum_bytes = limits.maximum_bytes;
    let original =
        super::auto_training_tables::prepare_identity_rows(&source, &bounded_request, &rows)?;
    let temporal_units =
        super::auto_training_tables::pipeline_time_units(&plan.pipeline, &source.temporal_units);
    let configured = fitted.limits.clone();
    // The single causal pass holds the history of every window group in the table at once,
    // so the table bounds groups and history here; maximum_bytes still caps the state.
    let window_steps = plan
        .pipeline
        .steps
        .iter()
        .filter(|step| matches!(step, FeatureStep::WindowFeatures { .. }))
        .count();
    let table_state = window_steps.saturating_mul(rows.len());
    fitted.limits.maximum_groups = configured
        .maximum_groups
        .max(table_state.min(MAXIMUM_WINDOW_GROUPS));
    fitted.limits.maximum_history_rows = configured
        .maximum_history_rows
        .max(table_state.min(MAXIMUM_WINDOW_HISTORY_ROWS));
    fitted.limits.maximum_duration_ms = deadline.remaining_ms()?;
    let transformed =
        super::auto_training_tables::transform_feature_rows(&fitted, &rows, &temporal_units);
    fitted.limits = configured;
    let transformed = transformed?;
    if transformed.len() != rows.len() {
        return Err(anyhow!("Feature pipeline changed row membership"));
    }
    deadline.remaining_ms()?;
    let mut prepared_request = bounded_request;
    let FeatureSource::Tabular { options } = &mut prepared_request.mapping.features else {
        unreachable!()
    };
    options.numeric_columns = plan.numeric_columns.clone();
    options.categorical_columns = plan.categorical_columns.clone();
    options.standardize = plan.standardize;
    options.imputation = plan.imputation.clone();
    let mut prepared =
        super::auto_training_tables::prepare_rows_owned(source, &prepared_request, transformed)?;
    prepared.raw_content_digests = original.raw_content_digests;
    prepared.raw_rows = Arc::new(rows);
    let profiled: BTreeSet<String> =
        super::auto_training_tables::profiled_columns(&request.mapping)
            .into_iter()
            .chain(super::auto_training_tables::profiled_columns(
                &prepared_request.mapping,
            ))
            .collect();
    prepared.profile = super::auto_training_tables::profile_training_rows(
        &prepared.rows,
        &prepared.split.train,
        &profiled.into_iter().collect::<Vec<_>>(),
    )?;
    let projection = if let Some(components) = plan.pca_components {
        let inputs: Vec<_> = prepared
            .samples
            .iter()
            .map(|sample| sample.input.clone())
            .collect();
        let projection = flow_like_ml_native::reduction::FittedPca::fit(
            &inputs,
            &prepared.split.train,
            components,
        )?;
        let projected =
            projection.transform_rows(&inputs, request.budget.maximum_tensor_elements)?;
        let prepared_rows = Arc::make_mut(&mut prepared.rows);
        for ((sample, row), input) in prepared
            .samples
            .iter_mut()
            .zip(prepared_rows.iter_mut())
            .zip(projected)
        {
            for (component, value) in input.values.iter().enumerate() {
                let name = format!("__ml_pca_{}", component + 1);
                if row
                    .as_object_mut()
                    .ok_or_else(|| anyhow!("Feature row must be an object"))?
                    .insert(name, flow_like_types::json::json!(value))
                    .is_some()
                {
                    return Err(anyhow!("Source column collides with generated PCA output"));
                }
            }
            sample.input = input;
        }
        prepared.spec.input_shape = vec![components];
        Some(projection)
    } else {
        None
    };
    let mut output_bytes = 0usize;
    for row in prepared.rows.iter() {
        if row.as_object().map_or(0, |row| row.len()) > limits.maximum_columns {
            return Err(anyhow!("Prepared features exceed the column budget"));
        }
        output_bytes = output_bytes
            .checked_add(super::auto_training_tables::serialized_json_size(row)?)
            .ok_or_else(|| anyhow!("Feature byte budget overflow"))?;
        if output_bytes > limits.maximum_bytes {
            return Err(anyhow!("Prepared features exceed the byte budget"));
        }
    }
    prepared.preprocessing = TablePreprocessing::Engineered {
        pipeline: fitted,
        preprocessing: Box::new(prepared.preprocessing),
        projection,
        plan: Some(Box::new(plan.clone())),
        temporal_units,
    };
    prepared.feature_sources = sources;
    prepared.digest = flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(&(
        &prepared.source.reference,
        &prepared.feature_sources,
        &prepared.samples,
        &prepared.preprocessing,
        &prepared.split,
    ))?);
    Ok(prepared)
}

#[cfg(feature = "execute")]
pub async fn prepare_feature_dataset(
    context: &mut ExecutionContext,
    request: &TableDatasetRequest,
    plan: &FeaturePlan,
    sources: &[FeatureSourceInput],
) -> Result<PreparedTableDataset> {
    let source = super::auto_training_tables::pin_table_source(context, &request.source).await?;
    let pinned = pin_feature_sources(context, sources).await?;
    prepare_pinned_feature_dataset(context, source, request, plan, pinned).await
}

#[cfg(feature = "execute")]
pub async fn pin_feature_sources(
    context: &mut ExecutionContext,
    sources: &[FeatureSourceInput],
) -> Result<Vec<PinnedFeatureSource>> {
    if sources.len() > 32 {
        return Err(anyhow!("At most 32 authorized side sources are supported"));
    }
    let mut pinned = Vec::new();
    let mut aliases = HashSet::new();
    for input in sources {
        if input.alias.is_empty() || input.alias.len() > 128 || !aliases.insert(&input.alias) {
            return Err(anyhow!("Feature source aliases must be unique and bounded"));
        }
        pinned.push(PinnedFeatureSource {
            alias: input.alias.clone(),
            source: super::auto_training_tables::pin_table_source(context, &input.source).await?,
            allowed_columns: input.allowed_columns.clone(),
        });
    }
    Ok(pinned)
}

#[cfg(feature = "execute")]
pub async fn prepare_pinned_feature_dataset(
    context: &mut ExecutionContext,
    mut source: PinnedTableSource,
    request: &TableDatasetRequest,
    plan: &FeaturePlan,
    mut sources: Vec<PinnedFeatureSource>,
) -> Result<PreparedTableDataset> {
    use super::auto_training_tables::{
        prepared_columns, read_columns, reopen_pinned_source, serialized_json_size,
    };
    source.database = reopen_pinned_source(context, &source).await?;
    let columns = prepared_columns(&request.mapping);
    let rows = read_columns(context, &source, &request.budget, &columns).await?;
    let mut side_rows = BTreeMap::new();
    let mut aggregate_bytes = 0usize;
    for side in &mut sources {
        side.source.database = reopen_pinned_source(context, &side.source).await?;
        let mut budget = request.budget.clone();
        budget.maximum_rows = budget.maximum_rows.min(plan.limits.maximum_join_rows);
        budget.maximum_bytes = budget.maximum_bytes.min(plan.limits.maximum_bytes);
        let rows = read_columns(context, &side.source, &budget, &side.allowed_columns).await?;
        aggregate_bytes = aggregate_bytes
            .checked_add(serialized_json_size(&rows)?)
            .ok_or_else(|| anyhow!("Side source byte count overflow"))?;
        if aggregate_bytes > plan.limits.maximum_bytes {
            return Err(anyhow!("Combined feature sources exceed the byte budget"));
        }
        if side_rows.insert(side.alias.clone(), rows).is_some() {
            return Err(anyhow!("Duplicate feature source alias"));
        }
    }
    let request = request.clone();
    let plan = plan.clone();
    tokio::task::spawn_blocking(move || {
        prepare_feature_rows_owned(source, &request, rows, &plan, sources, &side_rows)
    })
    .await?
}

#[cfg(feature = "execute")]
pub async fn transform_and_materialize(
    context: &mut ExecutionContext,
    request: FeatureTransformRequest,
) -> Result<FeatureTransformResult> {
    let prepared =
        prepare_feature_dataset(context, &request.dataset, &request.plan, &request.sources).await?;
    let tables = super::auto_training_tables::materialize_prepared_tables(
        context,
        &prepared,
        &request.output_id,
        &request.tables,
    )
    .await?;
    Ok(FeatureTransformResult { prepared, tables })
}

#[cfg(feature = "training")]
pub async fn derive_pipeline_candidate(
    context: &mut ExecutionContext,
    id: &str,
    plan: FeaturePlan,
) -> Result<()> {
    let repo = super::auto_training::experiment_repository(context, id)?;
    let experiment = repo.get_experiment(id)?;
    let original_limits: FeatureLimits = experiment
        .request
        .context
        .pointer("/feature_plan/limits")
        .filter(|value| !value.is_null())
        .map(|value| flow_like_types::json::from_value(value.clone()))
        .transpose()?
        .unwrap_or_default();
    let limits = &plan.limits;
    if limits.maximum_rows > original_limits.maximum_rows
        || limits.maximum_bytes > original_limits.maximum_bytes
        || limits.maximum_columns > original_limits.maximum_columns
        || limits.maximum_expression_nodes > original_limits.maximum_expression_nodes
        || limits.maximum_history_rows > original_limits.maximum_history_rows
        || limits.maximum_groups > original_limits.maximum_groups
        || limits.maximum_join_rows > original_limits.maximum_join_rows
        || limits.maximum_duration_ms > original_limits.maximum_duration_ms
    {
        return Err(anyhow!(
            "A feature proposal cannot increase the operator's transformation limits"
        ));
    }
    let request: TableDatasetRequest =
        flow_like_types::json::from_value(experiment.request.context["dataset_request"].clone())?;
    let source: PinnedTableSource =
        flow_like_types::json::from_value(experiment.request.context["pinned_source"].clone())?;
    let sources: Vec<PinnedFeatureSource> = flow_like_types::json::from_value(
        experiment
            .request
            .context
            .get("feature_sources")
            .cloned()
            .unwrap_or_else(|| flow_like_types::json::json!([])),
    )?;
    let prepared =
        prepare_pinned_feature_dataset(context, source, &request, &plan, sources).await?;
    super::auto_training::register_prepared_candidate(context, id, prepared).await
}

#[crate::register_node]
#[derive(Default)]
pub struct DeriveColumnsNode;

#[crate::register_node]
#[derive(Default)]
pub struct WindowFeaturesNode;

#[crate::register_node]
#[derive(Default)]
pub struct JoinSourcesNode;

macro_rules! feature_node {
    ($node:ident,$id:literal,$name:literal,$function:literal,$variant:ident) => {
        #[flow_like_types::async_trait]
        impl NodeLogic for $node {
            fn get_node(&self) -> Node {
                let mut node =
                    super::operation_node::<FeatureTransformRequest, FeatureTransformResult>(
                        $id,
                        $name,
                        "Create named feature columns in pinned derived tables",
                        $function,
                        "AI/ML/Feature Engineering",
                    );
                node.set_version(1);
                node
            }
            async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
                #[cfg(feature = "execute")]
                {
                    context.deactivate_exec_pin("exec_out").await?;
                    let request: FeatureTransformRequest = context.evaluate_pin("request").await?;
                    if !request
                        .plan
                        .pipeline
                        .steps
                        .iter()
                        .any(|step| matches!(step, FeatureStep::$variant { .. }))
                    {
                        return Err(anyhow!(
                            "The feature plan does not contain this node's operation"
                        ));
                    }
                    let result = transform_and_materialize(context, request).await?;
                    context
                        .set_pin_value("result", flow_like_types::json::json!(result))
                        .await?;
                    context.activate_exec_pin("exec_out").await?;
                    Ok(())
                }
                #[cfg(not(feature = "execute"))]
                {
                    let _ = context;
                    Err(anyhow!("Feature table execution requires a native build"))
                }
            }
        }
    };
}
feature_node!(
    DeriveColumnsNode,
    "ml_derive_columns",
    "Derive Columns",
    "deriveColumns",
    DeriveColumns
);
feature_node!(
    WindowFeaturesNode,
    "ml_window_features",
    "Window Features",
    "windowFeatures",
    WindowFeatures
);
feature_node!(
    JoinSourcesNode,
    "ml_join_sources",
    "Join Sources",
    "joinSources",
    JoinSources
);

#[crate::register_node]
#[derive(Default)]
pub struct FitPreprocessingNode;
#[flow_like_types::async_trait]
impl NodeLogic for FitPreprocessingNode {
    fn get_node(&self) -> Node {
        let mut node = super::operation_node::<FitPreprocessingRequest, PreparedTableDataset>(
            "ml_fit_preprocessing",
            "Fit Preprocessing",
            "Fit imputation, encoding and scaling on training rows",
            "fitPreprocessing",
            "AI/ML/Feature Engineering",
        );
        node.set_version(1);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "execute")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: FitPreprocessingRequest = context.evaluate_pin("request").await?;
            let result =
                prepare_feature_dataset(context, &request.dataset, &request.plan, &request.sources)
                    .await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "execute"))]
        {
            let _ = context;
            Err(anyhow!("Feature preprocessing requires a native build"))
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct MaterializeDatasetNode;
#[flow_like_types::async_trait]
impl NodeLogic for MaterializeDatasetNode {
    fn get_node(&self) -> Node {
        let mut node = super::operation_node::<MaterializeDatasetRequest, Vec<MaterializedTable>>(
            "ml_materialize_dataset",
            "Materialize Dataset",
            "Write prepared partitions with their mapped source columns and generated named columns",
            "materializeDataset",
            "AI/ML/Feature Engineering",
        );
        node.set_version(1);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "execute")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: MaterializeDatasetRequest = context.evaluate_pin("request").await?;
            let result = super::auto_training_tables::materialize_prepared_tables(
                context,
                &request.prepared,
                &request.output_id,
                &request.tables,
            )
            .await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "execute"))]
        {
            let _ = context;
            Err(anyhow!("Feature materialization requires a native build"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preparation_deadline_spans_fit_and_transform() {
        let started = Instant::now()
            .checked_sub(std::time::Duration::from_millis(50))
            .unwrap();
        let spent = PreparationDeadline {
            started,
            budget_ms: 40,
        };
        assert!(
            spent
                .remaining_ms()
                .unwrap_err()
                .to_string()
                .contains("40 ms duration budget")
        );
        let open = PreparationDeadline {
            started,
            budget_ms: 60_000,
        };
        let remaining = open.remaining_ms().unwrap();
        assert!(remaining <= 59_950 && remaining > 0);
    }
}
