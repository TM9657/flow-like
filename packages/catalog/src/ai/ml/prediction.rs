//! Node for Making Predictions with MLModels
//!
//! This node loads a dataset (currently from a Database), transforms it into a prediction dataset,
//! and uses the trained model to make predictions.
//!
//! Adds / upserts predictions back into the Database.

use crate::ai::ml::{MAX_ML_PREDICTION_RECORDS, MLPrediction, NodeMLModel, make_new_field};
use crate::data::db::vector::NodeDBConnection;
use flow_like::flow::pin::ValueType;
use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic, remove_pin_by_name},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::arrow_schema::Schema;
use flow_like_storage::databases::vector::VectorStore;
use flow_like_storage::lancedb::table::NewColumnTransform;
use flow_like_types::{Result, Value, anyhow, async_trait, json::json};
use std::collections::HashSet;
use std::sync::Arc;

#[crate::register_node]
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
            "Data Source (DB, Vector, CSV, ...)",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec!["Database".to_string(), "Vector".to_string()]) // , "CSV".to_string()
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
                let predictions_col: String = context.evaluate_pin("predictions_col").await?;

                // fetch database
                let database = node_database.load(context).await?.db.clone();

                // fetch records
                let t0 = std::time::Instant::now();
                let (mut records, existing_cols) = {
                    let database = database.read().await;
                    let schema = database.schema().await?;
                    let existing_cols: HashSet<String> =
                        schema.fields.iter().map(|f| f.name().clone()).collect();
                    if !existing_cols.contains(&records_col) {
                        return Err(anyhow!(format!(
                            "Database doesn't contain input column `{}`!",
                            records_col
                        )));
                    }
                    let records = database
                        .filter(
                            "true",
                            Some(vec![records_col.to_string()]),
                            MAX_ML_PREDICTION_RECORDS,
                            0,
                        )
                        .await?;
                    (records, existing_cols)
                }; // drop read guard
                context.log_message(
                    &format!(
                        "Loaded {} records from database with columns {:?}",
                        records.len(),
                        &existing_cols
                    ),
                    LogLevel::Debug,
                );
                let elapsed = t0.elapsed();
                context.log_message(&format!("Fetch records (db): {elapsed:?}"), LogLevel::Debug);

                // predict
                let t0 = std::time::Instant::now();
                {
                    let model = node_model.get_model(context).await?;
                    let model_guard = model.lock().await;
                    model_guard.predict_on_values(&mut records, &records_col, &predictions_col)?;
                }; // drop model
                let elapsed = t0.elapsed();
                context.log_message(&format!("Predict: {elapsed:?}"), LogLevel::Debug);

                // upsert
                let t0 = std::time::Instant::now();
                {
                    let mut database = database.write().await;
                    if !existing_cols.contains(&predictions_col) {
                        // add new column for predictions
                        let probe = records.first().ok_or_else(|| anyhow!("Got No Records!"))?;
                        let new_field = make_new_field(probe, &predictions_col)?;
                        let schema = Schema::new(vec![new_field]);
                        database
                            .add_columns(NewColumnTransform::AllNulls(schema.into()), None)
                            .await?;
                        context.log_message(
                            &format!("Added {} as new column", predictions_col),
                            LogLevel::Debug,
                        );
                    }
                    // upsert records with predictions
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
            "Vector" => {
                // load vector as dataset
                let vector: Vec<f64> = context.evaluate_pin("vector").await?;

                let t0 = std::time::Instant::now();
                let prediction = {
                    let model = node_model.get_model(context).await?;
                    let model_guard = model.lock().await;
                    model_guard.predict_on_vector(vector)?
                }; // drop model
                let elapsed = t0.elapsed();
                context.log_message(&format!("Predict: {elapsed:?}"), LogLevel::Debug);

                // set outputs
                context
                    .set_pin_value("prediction", json!(prediction))
                    .await?;
            }
            _ => return Err(anyhow!("Datasource Not Implemented")),
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
            if node.get_pin_by_name("predictions_col").is_none() {
                node.add_input_pin(
                    "predictions_col",
                    "Output Col",
                    "Column that should be added for predictions",
                    VariableType::String,
                );
            }
            if node.get_pin_by_name("database_out").is_none() {
                node.add_output_pin(
                    "database_out",
                    "Database",
                    "Database Connection (Updated)",
                    VariableType::Struct,
                )
                .set_schema::<NodeDBConnection>()
                .set_options(PinOptions::new().set_enforce_schema(true).build());
            }
            remove_pin_by_name(node, "vector");
            remove_pin_by_name(node, "prediction");
        } else if source_pin == *"Vector" {
            if node.get_pin_by_name("vector").is_none() {
                node.add_input_pin("vector", "Vector", "Vector (1d Array)", VariableType::Float)
                    .set_value_type(ValueType::Array);
            }
            if node.get_pin_by_name("prediction").is_none() {
                node.add_output_pin(
                    "prediction",
                    "Prediction",
                    "Model Prediction as Struct",
                    VariableType::Struct,
                )
                .set_schema::<MLPrediction>();
            }
            remove_pin_by_name(node, "database");
            remove_pin_by_name(node, "records");
            remove_pin_by_name(node, "predictions_col");
            remove_pin_by_name(node, "database_out");
        } else {
            node.error = Some("Datasource Not Implemented".to_string());
            return;
        }
    }
}
