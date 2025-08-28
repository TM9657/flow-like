use crate::ai::ml::{
    MAX_RECORDS, MLDataset, MLModel, NodeMLModel, update_records_with_predictions,
    values_to_dataset,
};
use crate::storage::{db::vector::NodeDBConnection, path::FlowPath};
use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::databases::vector::VectorStore;
use flow_like_storage::lancedb::table::NewColumnTransform;
use flow_like_types::{Result, Value, anyhow, async_trait, json::json};
use linfa::composing::MultiClassModel;
use linfa::traits::Predict;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Default)]
pub struct MLPredictNode {}

impl MLPredictNode {
    pub fn new() -> Self {
        MLPredictNode {}
    }
}

#[async_trait]
impl NodeLogic for MLPredictNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ml_predict",
            "Predict",
            "Predict with Machine Learning Model",
            "AI/ML",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.add_input_pin("exec_in", "Input", "Start Fitting", VariableType::Execution);

        node.add_input_pin(
            "model",
            "Model",
            "Trained KMeans Clustering Model",
            VariableType::Struct,
        )
        .set_schema::<NodeMLModel>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "source",
            "Data Source",
            "Data Source (DB or CSV)",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec!["Database".to_string(), "CSV".to_string()])
                .build(),
        )
        .set_default_value(Some(json!("Database")));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Done Fitting Model",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        // fetch inputs
        context.deactivate_exec_pin("exec_out").await?;
        let source: String = context.evaluate_pin("source").await?;
        let node_model: NodeMLModel = context.evaluate_pin("model").await?;

        // load dataset
        match source.as_str() {
            "Database" => {
                // fetch additional inputs
                let node_database: NodeDBConnection = context.evaluate_pin("database").await?;
                let records_col: String = context.evaluate_pin("records").await?;
                let predictions_col: String = context.evaluate_pin("predictions").await?;

                // fetch database
                let database = node_database
                    .load(context, &node_database.cache_key)
                    .await?
                    .db
                    .clone();

                // fetch records
                let t0 = std::time::Instant::now();
                let records = {
                    let database = database.read().await;
                    database
                        .filter("true", Some(vec![records_col.to_string()]), MAX_RECORDS, 0)
                        .await?
                }; // drop read guard
                let existing_cols: HashSet<&String> = match records.first() {
                    Some(Value::Object(map)) => map.keys().collect(),
                    _ => HashSet::new(),
                };
                context.log_message(
                    &format!("Loaded {} records from database with columns {:?}", records.len(), &existing_cols),
                    LogLevel::Debug,
                );
                let elapsed = t0.elapsed();
                context.log_message(&format!("Fetch records (db): {elapsed:?}"), LogLevel::Debug);

                // make dataset -> load as unlabeld dataset as we want to predict the labels
                let t0 = std::time::Instant::now();
                let ds = match values_to_dataset(&records, &records_col, None, None)? {
                    MLDataset::Unlabeled(ds) => ds,
                    _ => return Err(anyhow!("Invalid Dataset Format!")),
                };
                let elapsed = t0.elapsed();
                context.log_message(
                    &format!("Preprocess data (ds): {elapsed:?}"),
                    LogLevel::Debug,
                );

                // predict
                let t0 = std::time::Instant::now();
                let predictions: ndarray::ArrayBase<
                    ndarray::OwnedRepr<_>,
                    ndarray::Dim<[usize; 1]>,
                > = {
                    let model = node_model.get_model(context).await?;
                    let model_guard = model.lock().await;
                    match &*model_guard {
                        MLModel::KMeans(model) => model.predict(&ds).mapv(|x| x as f64),
                        MLModel::LinearRegression(model) => model.predict(&ds),
                        MLModel::SVMMultiClass(models) => {
                            let model = MultiClassModel::from_iter(models.clone());
                            model.predict(&ds).mapv(|x| x as f64)
                        }
                        _ => return Err(anyhow!("Unknown Machine Learning Model!")),
                    }
                };
                let elapsed = t0.elapsed();
                context.log_message(&format!("Predict: {elapsed:?}"), LogLevel::Debug);
                context.log_message(
                    &format!("Output Array Dims: {}", predictions.dim()),
                    LogLevel::Debug,
                );

                // upsert
                let t0 = std::time::Instant::now();
                {
                    let mut database = database.write().await;
                    if !existing_cols.contains(&predictions_col) {
                        // add new column for predictions
                        let new_col = vec![(predictions_col.to_string(), "CAST(NULL AS DOUBLE)".to_string())];
                        database
                            .add_columns(NewColumnTransform::SqlExpressions(new_col), None)
                            .await?;
                        context.log_message(&format!("Added {} as new column", predictions_col), LogLevel::Debug);
                    }
                    // update records
                    let t0 = std::time::Instant::now();
                    let records = update_records_with_predictions(records, predictions, &predictions_col)?;
                    let elapsed = t0.elapsed();
                    context.log_message(&format!("Update records: {elapsed:?}"), LogLevel::Debug);

                    database.upsert(records, records_col).await?;
                } // drop database read/write guard
                let elapsed = t0.elapsed();
                context.log_message(&format!("Update database: {elapsed:?}"), LogLevel::Debug);

                // set output
                let database_value: Value = flow_like_types::json::to_value(&node_database)?;
                context
                    .set_pin_value("database_out", database_value)
                    .await?;
            }
            _ => return Err(anyhow!("Datasource not implemented")),
        };

        // set outputs
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        let source_pin: String = node
            .get_pin_by_name("source")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| flow_like_types::json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        if source_pin == *"Database" {
            if node.get_pin_by_name("database").is_none() {
                node.add_input_pin(
                    "database",
                    "Database",
                    "Database Connection",
                    VariableType::Struct,
                )
                .set_schema::<NodeDBConnection>()
                .set_options(PinOptions::new().set_enforce_schema(true).build());
            }
            if node.get_pin_by_name("records").is_none() {
                node.add_input_pin(
                    "records",
                    "Input Col",
                    "Column containing records to predict on",
                    VariableType::String,
                )
                .set_default_value(Some(json!("vector")));
            }
            if node.get_pin_by_name("predictions").is_none() {
                node.add_input_pin(
                    "predictions",
                    "Output Col",
                    "Column that should be added for predictions",
                    VariableType::String,
                )
                .set_default_value(Some(json!("prediction")));
            }
            if node.get_pin_by_name("database_out").is_none() {
                node.add_output_pin(
                    "database_out",
                    "Database",
                    "Updated Database Connection",
                    VariableType::Struct,
                )
                .set_schema::<NodeDBConnection>()
                .set_options(PinOptions::new().set_enforce_schema(true).build());
            }
            remove_pin(node, "csv");
        } else {
            if node.get_pin_by_name("csv").is_none() {
                node.add_input_pin("csv", "CSV", "CSV Path", VariableType::Struct)
                    .set_schema::<FlowPath>()
                    .set_options(PinOptions::new().set_enforce_schema(true).build());
            }
            remove_pin(node, "database");
            remove_pin(node, "records");
            remove_pin(node, "predictions");
            remove_pin(node, "database_out");
        }
    }
}

fn remove_pin(node: &mut Node, name: &str) {
    if let Some(pin) = node.get_pin_by_name(name) {
        node.pins.remove(&pin.id.clone());
    }
}
