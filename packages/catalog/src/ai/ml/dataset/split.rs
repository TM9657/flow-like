use crate::data::db::vector::NodeDBConnection;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::arrow_utils::record_batch_to_value;
use flow_like_storage::databases::vector::VectorStore;
use flow_like_storage::lancedb::query::ExecutableQuery;
use flow_like_types::rand::{self, Rng};
use flow_like_types::{Result, async_trait, json::json};
use futures::TryStreamExt;

#[crate::register_node]
#[derive(Default)]
pub struct SplitDatasetNode {}

impl SplitDatasetNode {
    pub fn new() -> Self {
        SplitDatasetNode {}
    }
}

#[async_trait]
impl NodeLogic for SplitDatasetNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_ml_dataset_split",
            "Split Dataset",
            "Split a dataset into training and testing subsets",
            "AI/ML/Dataset",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Start Splitting",
            VariableType::Execution,
        );

        node.add_input_pin(
            "split",
            "Split",
            "Split Ratio (Train/Test)",
            VariableType::Float,
        )
        .set_options(PinOptions::new().set_range((0.0, 1.0)).build())
        .set_default_value(Some(json!(0.8)));

        node.add_input_pin(
            "source",
            "Data Source",
            "Data Source (DB or CSV)",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "train",
            "Training Database",
            "Training Data",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "test",
            "Test Database",
            "Testing Data",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

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
        let source: NodeDBConnection = context.evaluate_pin("source").await?;
        let test: NodeDBConnection = context.evaluate_pin("test").await?;
        let train: NodeDBConnection = context.evaluate_pin("train").await?;
        let probability: f64 = context.evaluate_pin("split").await?;

        let source = source.load(context).await?;
        let test = test.load(context).await?;
        let train = train.load(context).await?;

        let source_db = source.db.read().await.clone();
        let mut test_db = test.db.read().await.clone();
        let mut train_db = train.db.read().await.clone();

        let source_table = source_db.raw().await?;
        let query = source_table.query();
        let mut item_stream = query.execute().await?;

        while let Ok(Some(items)) = item_stream.try_next().await {
            let items = record_batch_to_value(&items)?;

            let mut train_items = Vec::with_capacity(items.len());
            let mut test_items = Vec::with_capacity(items.len());

            {
                let mut rng = rand::rng();

                for item in items {
                    let random_bool = rng.random_bool(probability);

                    if random_bool {
                        train_items.push(item);
                    } else {
                        test_items.push(item);
                    }
                }
            }

            if !train_items.is_empty() {
                train_db.insert(train_items).await?;
            }
            if !test_items.is_empty() {
                test_db.insert(test_items).await?;
            }
        }

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
