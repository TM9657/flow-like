use crate::ai::ml::{MLModel, NodeMLModel, values_to_dataset};
use crate::storage::{db::vector::NodeDBConnection, path::FlowPath};
use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::{Pin, PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::databases::vector::VectorStore;
use flow_like_types::{Result, Value, anyhow, async_trait, json::json};
use linfa::traits::Fit;
use linfa_clustering::KMeans;
use linfa_nn::distance::L2Dist;
use ndarray::{Array2, ArrayBase, Dim, OwnedRepr};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Default)]
pub struct FitKMeansNode {}

impl FitKMeansNode {
    pub fn new() -> Self {
        FitKMeansNode {}
    }
}

#[async_trait]
impl NodeLogic for FitKMeansNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "fit_kmeans",
            "Train KMeans",
            "Fit/Train KMeans Clustering Algorithm",
            "AI/ML/Clustering",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.add_input_pin("exec_in", "Input", "Start Fitting", VariableType::Execution);

        node.add_input_pin(
            "cluster",
            "Cluster",
            "Number of Clusters",
            VariableType::Integer,
        )
        .set_options(PinOptions::new().set_range((1., 100.)).build())
        .set_default_value(Some(json!(2)));

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

        //node.add_input_pin(
        //    "targets",
        //    "Target Column",
        //    "Column containing targets to train on / to evaluate",
        //    VariableType::String,
        //)
        //.set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Done Fitting Model",
            VariableType::Execution,
        );

        node.add_output_pin(
            "model",
            "Model",
            "Trained KMeans Clustering Model",
            VariableType::Struct,
        )
        .set_schema::<NodeMLModel>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        // fetch inputs
        context.deactivate_exec_pin("exec_out").await?;
        let source: String = context.evaluate_pin("source").await?;
        let n_clusters: usize = context.evaluate_pin("cluster").await?;

        // load dataset
        let t0 = std::time::Instant::now();
        let ds = match source.as_str() {
            "Database" => {
                let database: NodeDBConnection = context.evaluate_pin("database").await?;
                let records_col: String = context.evaluate_pin("records").await?;
                //let should_update: bool = context.evaluate_pin("update").await?;

                // fetch records
                let records = {
                    let database = database
                        .load(context, &database.cache_key)
                        .await?
                        .db
                        .clone();
                    let database = database.read().await;
                    database
                        .filter("true", Some(vec![records_col.to_string()]), 100, 0)
                        .await?
                }; // drop db
                context.log_message(
                    &format!("Loaded {} records from database", records.len()),
                    LogLevel::Debug,
                );

                // make dataset
                values_to_dataset(&records, &records_col)?
            }
            _ => return Err(anyhow!("Datasource not implemented")),
        };

        let elapsed = t0.elapsed();
        context.log_message(&format!("Preprocess data: {elapsed:?}"), LogLevel::Debug);

        // train model
        let t0 = std::time::Instant::now();
        let kmeans: KMeans<f64, L2Dist> = KMeans::params(n_clusters).fit(&ds)?;
        let elapsed = t0.elapsed();
        context.log_message(&format!("Fit model: {elapsed:?}"), LogLevel::Debug);

        // set outputs
        let model = MLModel::KMeans(kmeans);
        let node_model = NodeMLModel::new(context, model).await;
        context.set_pin_value("model", json!(node_model)).await?;
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
                    "Train Column",
                    "Column containing records to train on",
                    VariableType::String,
                )
                .set_default_value(Some(json!("vector")));
            }
            // if node.get_pin_by_name("update").is_none() {
            //     node.add_input_pin(
            //         "update",
            //         "Update DB?",
            //         "Update database with predictions on training data?",
            //         VariableType::Boolean,
            //     )
            //     .set_default_value(Some(json!(false)));
            // }
            // if node.get_pin_by_name("database_out").is_none() {
            //     node.add_output_pin(
            //         "database_out",
            //         "Database",
            //         "Updated Database Connection",
            //         VariableType::Struct,
            //     )
            //     .set_schema::<NodeDBConnection>()
            //     .set_options(PinOptions::new().set_enforce_schema(true).build());
            // }
            remove_pin(node, "csv");
        } else {
            if node.get_pin_by_name("csv").is_none() {
                node.add_input_pin("csv", "CSV", "CSV Path", VariableType::Struct)
                    .set_schema::<FlowPath>()
                    .set_options(PinOptions::new().set_enforce_schema(true).build());
            }
            remove_pin(node, "database");
            remove_pin(node, "records");
            //remove_pin(node, "update");
            //remove_pin(node, "database_out");
        }
    }
}

fn remove_pin(node: &mut Node, name: &str) {
    if let Some(pin) = node.get_pin_by_name(name) {
        node.pins.remove(&pin.id.clone());
    }
}
