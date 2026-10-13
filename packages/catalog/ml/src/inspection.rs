pub mod auto_training;
pub mod auto_training_legacy;
pub mod auto_training_setup;
pub mod auto_training_tables;
pub mod cancellation;
pub mod compute;
pub mod data;
pub mod feature_engineering;
pub mod image;
pub mod learning;
pub mod ledger_transfer;
pub mod lifecycle;
pub mod metrics;
pub mod model_nodes;
pub mod models;
pub mod pretrained;
pub mod published;
#[cfg(feature = "training")]
pub(crate) mod publisher;
pub mod sam2;
pub mod sensors;

use flow_like::flow::{node::Node, pin::PinOptions, variable::VariableType};
use schemars::JsonSchema;

fn operation_node<I: JsonSchema + serde::Serialize, O: JsonSchema + serde::Serialize>(
    id: &str,
    name: &str,
    description: &str,
    function: &str,
    category: &str,
) -> Node {
    let mut node = Node::new(id, name, description, category);
    node.set_flowscript_name("inspection", function);
    node.add_icon("/flow/icons/chart-network.svg");
    node.add_input_pin(
        "exec_in",
        "Input",
        "Run this operation",
        VariableType::Execution,
    );
    node.add_input_pin(
        "request",
        "Configuration",
        "Inputs and configuration",
        VariableType::Struct,
    )
    .set_schema::<I>()
    .set_options(PinOptions::new().set_enforce_schema(true).build());
    node.add_output_pin(
        "exec_out",
        "Done",
        "Operation completed",
        VariableType::Execution,
    );
    let result = node.add_output_pin("result", "Result", "Operation result", VariableType::Struct);
    let schema = flow_like_types::json::json!(schemars::schema_for!(O));
    let is_array = schema.get("type").and_then(|v| v.as_str()) == Some("array");
    let mut item = if is_array {
        schema
            .get("items")
            .cloned()
            .unwrap_or(flow_like_types::json::json!({}))
    } else {
        schema.clone()
    };
    let data_type = match item.get("type").and_then(|v| v.as_str()) {
        Some("boolean") => VariableType::Boolean,
        Some("integer") => VariableType::Integer,
        Some("number") => VariableType::Float,
        Some("string") => VariableType::String,
        _ => VariableType::Struct,
    };
    result.set_data_type(data_type.clone());
    if is_array {
        result.set_value_type(flow_like::flow::pin::ValueType::Array);
    }
    if data_type == VariableType::Struct {
        if item == flow_like_types::json::json!(true) {
            result.set_open_schema();
        } else {
            if is_array {
                if let Some(defs) = schema.get("$defs") {
                    item["$defs"] = defs.clone();
                }
            }
            result.schema = flow_like_types::json::to_string(&item).ok();
        }
    }
    node
}

macro_rules! operation {
    ($node:ident, $id:literal, $name:literal, $description:literal, $function:literal, $category:literal, $input:ty, $output:ty, $handler:path $(, version = $version:literal)?) => {
        #[flow_like_types::async_trait]
        impl flow_like::flow::node::NodeLogic for $node {
            fn get_node(&self) -> flow_like::flow::node::Node {
                let node = super::operation_node::<$input, $output>(
                    $id,
                    $name,
                    $description,
                    $function,
                    $category,
                );
                $(
                    let mut node = node;
                    node.set_version($version);
                )?
                node
            }
            async fn run(
                &self,
                context: &mut flow_like::flow::execution::context::ExecutionContext,
            ) -> flow_like_types::Result<()> {
                #[cfg(feature = "execute")]
                {
                    context.deactivate_exec_pin("exec_out").await?;
                    let input: $input = context.evaluate_pin("request").await?;
                    let output: $output =
                        tokio::task::spawn_blocking(move || $handler(input)).await??;
                    context
                        .set_pin_value("result", flow_like_types::json::json!(output))
                        .await?;
                    context.activate_exec_pin("exec_out").await?;
                    Ok(())
                }
                #[cfg(not(feature = "execute"))]
                {
                    let _ = context;
                    Err(flow_like_types::anyhow!(
                        "ML execution requires the execute feature"
                    ))
                }
            }
        }
    };
}
pub(crate) use operation;

/// Awaits `work`, running `cancel` if the run's token fires or this future is dropped first.
/// The executor drops node futures on cancellation, so the drop path is the one that usually runs.
#[cfg(feature = "execute")]
pub(crate) async fn join_cancellable<T, C>(
    token: Option<flow_like_types::tokio_util::sync::CancellationToken>,
    cancel: C,
    mut work: tokio::task::JoinHandle<flow_like_types::Result<T>>,
) -> flow_like_types::Result<T>
where
    C: Fn() + Clone + Send,
{
    let guard = cancellation::CancelOnDrop::new(cancel.clone());
    let result = match token {
        Some(token) => tokio::select! {
            result = &mut work => result,
            _ = token.cancelled() => {
                cancel();
                work.await
            }
        },
        None => work.await,
    };
    guard.disarm();
    result?
}

#[cfg(feature = "execute")]
pub(crate) async fn run_cancellable_operation<I, O, H>(
    context: &mut flow_like::flow::execution::context::ExecutionContext,
    handler: H,
) -> flow_like_types::Result<()>
where
    I: serde::de::DeserializeOwned + Send + 'static,
    O: serde::Serialize + Send + 'static,
    H: FnOnce(I, cancellation::CancelFlag) -> flow_like_types::Result<O> + Send + 'static,
{
    context.deactivate_exec_pin("exec_out").await?;
    let input: I = context.evaluate_pin("request").await?;
    let flag = cancellation::CancelFlag::default();
    let worker = flag.clone();
    let work = tokio::task::spawn_blocking(move || handler(input, worker));
    let token = context.get_cancellation_token();
    let output = join_cancellable(token, move || flag.cancel(), work).await?;
    context
        .set_pin_value("result", flow_like_types::json::json!(output))
        .await?;
    context.activate_exec_pin("exec_out").await
}

/// `operation!` for handlers `fn(Input, CancelFlag) -> Result<Output>` that stop once the flag
/// is cancelled, which happens when the run is cancelled or the node is dropped.
macro_rules! cancellable_operation {
    ($node:ident, $id:literal, $name:literal, $description:literal, $function:literal, $category:literal, $input:ty, $output:ty, $handler:path $(, version = $version:literal)?) => {
        #[flow_like_types::async_trait]
        impl flow_like::flow::node::NodeLogic for $node {
            fn get_node(&self) -> flow_like::flow::node::Node {
                let node = super::operation_node::<$input, $output>(
                    $id,
                    $name,
                    $description,
                    $function,
                    $category,
                );
                $(
                    let mut node = node;
                    node.set_version($version);
                )?
                node
            }
            async fn run(
                &self,
                context: &mut flow_like::flow::execution::context::ExecutionContext,
            ) -> flow_like_types::Result<()> {
                #[cfg(feature = "execute")]
                {
                    super::run_cancellable_operation::<$input, $output, _>(context, $handler)
                        .await
                }
                #[cfg(not(feature = "execute"))]
                {
                    let _ = context;
                    Err(flow_like_types::anyhow!(
                        "ML execution requires the execute feature"
                    ))
                }
            }
        }
    };
}
pub(crate) use cancellable_operation;

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like::flow::pin::ValueType;
    #[test]
    fn generated_pin_metadata_preserves_array_and_scalar_types() {
        let node = operation_node::<Vec<f64>, Vec<f64>>("test", "Test", "Test", "test", "Test");
        let pin = node.get_pin_by_name("result").unwrap();
        assert_eq!(pin.data_type, VariableType::Float);
        assert_eq!(pin.value_type, ValueType::Array);
        let node = operation_node::<bool, bool>("test", "Test", "Test", "test", "Test");
        assert_eq!(
            node.get_pin_by_name("result").unwrap().data_type,
            VariableType::Boolean
        );
    }
    #[test]
    fn registered_inspection_nodes_have_unique_ids_and_valid_schemas() {
        let mut ids = std::collections::HashSet::new();
        for logic in crate::get_catalog() {
            let node = logic.get_node();
            assert!(
                ids.insert(node.name.clone()),
                "Duplicate catalog node {}",
                node.name
            );
            if !node.category.starts_with("AI/ML/") {
                continue;
            }
            for pin in node.pins.values() {
                if let Some(schema) = &pin.schema {
                    flow_like_types::json::from_str::<flow_like_types::Value>(schema).unwrap();
                }
            }
        }
        for required in [
            "ml_import_pretrained_vision",
            "ml_list_pretrained_vision",
            "ml_extract_pretrained_features",
            "ml_train_dino_v2",
            "ml_train_rtdetr_v2",
            "ml_train_rf_detr",
            "ml_train_dfine_nano",
            "ml_prompt_sam2",
            "ml_fine_tune_sam2",
            "ml_auto_train_tabular",
            "ml_auto_train_forecast",
            "ml_auto_train_vision",
            "ml_auto_train_anomaly",
            "ml_get_auto_training",
            "ml_step_auto_training",
            "ml_cancel_auto_training",
            "ml_resume_auto_training",
            "ml_finalize_auto_training",
            "ml_predict_auto_model",
            "ml_export_auto_model",
            "ml_derive_columns",
            "ml_window_features",
            "ml_join_sources",
            "ml_fit_preprocessing",
            "ml_materialize_dataset",
            "ml_create_learning_project",
            "ml_step_learning_project",
            "ml_observe_learning_sample",
            "ml_review_learning_sample",
            "ml_select_samples_for_review",
            "ml_analyze_model_errors",
            "ml_route_learning_project",
            "ml_promote_learning_model",
            "ml_rollback_learning_model",
            "ml_reject_learning_candidate",
        ] {
            assert!(
                ids.contains(required),
                "Missing auto training node {required}"
            );
        }
    }

    #[test]
    fn feature_extractor_outputs_and_inputs_share_one_schema() {
        use flow_like::flow::{node::NodeLogic, pin::Pin};
        let extractor = Pin::schema_string_for::<lifecycle::FeatureExtractorRef>();
        for producer in [
            lifecycle::ExtractPatchFeaturesNode.get_node(),
            pretrained::ExtractPretrainedFeaturesNode.get_node(),
            data::FeatureMapToPatchesNode.get_node(),
        ] {
            let pin = producer.get_pin_by_name("extractor").unwrap();
            assert_eq!(pin.schema, extractor, "{}", producer.name);
        }
        for consumer in [
            lifecycle::ScheduleTrainingNode.get_node(),
            lifecycle::StudentInferenceNode.get_node(),
            auto_training::PredictAutoModelNode.get_node(),
            data::FeatureMapToPatchesNode.get_node(),
        ] {
            let request = consumer.get_pin_by_name("request").unwrap();
            let schema: flow_like_types::Value =
                flow_like_types::json::from_str(request.schema.as_deref().unwrap()).unwrap();
            let definitions = schema["$defs"].as_object().unwrap();
            assert!(
                definitions.contains_key("FeatureExtractorRef"),
                "{} defines {:?}",
                consumer.name,
                definitions.keys().collect::<Vec<_>>()
            );
        }
    }
}

#[cfg(all(test, feature = "execute"))]
mod join_cancellable_tests {
    use super::{cancellation::CancelFlag, join_cancellable};
    use flow_like_types::{Result, anyhow, tokio_util::sync::CancellationToken};
    use futures::FutureExt;
    use std::time::{Duration, Instant};

    fn cancels(flag: &CancelFlag) -> impl Fn() + Clone + Send {
        let flag = flag.clone();
        move || flag.cancel()
    }

    pub(super) fn wait_for_cancel(flag: &CancelFlag) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if flag.is_cancelled() {
                return Err(anyhow!("worker stopped after cancellation"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_fired_token_cancels_the_work_and_returns_its_error() {
        let token = CancellationToken::new();
        token.cancel();
        let flag = CancelFlag::default();
        let worker = flag.clone();
        let work = tokio::task::spawn_blocking(move || wait_for_cancel(&worker));
        let error = join_cancellable(Some(token), cancels(&flag), work)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "worker stopped after cancellation");
    }

    #[tokio::test]
    async fn dropping_the_join_cancels_the_work() {
        let flag = CancelFlag::default();
        let worker = flag.clone();
        let (stopped, observed) = std::sync::mpsc::channel();
        let work = tokio::task::spawn_blocking(move || {
            let result = wait_for_cancel(&worker);
            let _ = stopped.send(result.is_err());
            result
        });
        let join = join_cancellable(Some(CancellationToken::new()), cancels(&flag), work);
        assert!(join.now_or_never().is_none());
        assert!(flag.is_cancelled());
        assert!(observed.recv_timeout(Duration::from_secs(30)).unwrap());
    }

    #[tokio::test]
    async fn returned_work_disarms_the_guard() {
        let flag = CancelFlag::default();
        let token = Some(CancellationToken::new());
        let work = tokio::task::spawn_blocking(|| Ok(7));
        assert_eq!(
            join_cancellable(token, cancels(&flag), work).await.unwrap(),
            7
        );
        let work = tokio::task::spawn_blocking(|| Err::<(), _>(anyhow!("handler failed")));
        let error = join_cancellable(None, cancels(&flag), work)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "handler failed");
        assert!(!flag.is_cancelled());
    }
}

#[cfg(all(test, feature = "execute"))]
mod cancellable_operation_tests {
    use super::{
        join_cancellable_tests::wait_for_cancel, models::TrainGradientBoostedTreesNode,
        run_cancellable_operation,
    };
    use ahash::AHashMap;
    use flow_like::{
        flow::{
            board::ExecutionStage,
            execution::{
                LogLevel, Run, context::ExecutionContext, internal_node::InternalNode,
                internal_pin::InternalPin,
            },
            node::NodeLogic,
        },
        profile::Profile,
        state::{FlowLikeConfig, FlowLikeState},
        utils::http::HTTPClient,
    };
    use flow_like_types::{
        Value,
        json::json,
        sync::{Mutex, RwLock},
        tokio_util::sync::CancellationToken,
    };
    use futures::FutureExt;
    use std::{
        sync::{Arc, Weak, mpsc},
        time::Duration,
    };

    fn boost_request(rows: usize, estimators: usize) -> Value {
        let features: Vec<Vec<f64>> = (0..rows).map(|row| vec![row as f64]).collect();
        let targets: Vec<f64> = (0..rows).map(|row| (row % 7) as f64).collect();
        json!({
            "rows": features,
            "targets": targets,
            "config": {
                "objective": "squared_error",
                "estimators": estimators,
                "max_depth": 16,
                "max_bins": 256,
                "min_leaf": 1,
                "learning_rate": 0.001,
                "l2": 1.0
            }
        })
    }

    /// A detached run of the Train Gradient Boosted Trees node with `request` on its input.
    async fn boost_context(request: Value) -> ExecutionContext {
        let logic: Arc<dyn NodeLogic> = Arc::new(TrainGradientBoostedTreesNode);
        let node = logic.get_node();
        let pins = node
            .pins
            .values()
            .map(|pin| (pin.id.clone(), Arc::new(InternalPin::new(pin, false))))
            .collect();
        let node = Arc::new(InternalNode::new(node, pins, logic, AHashMap::new()));
        node.get_pin_by_name("request")
            .await
            .unwrap()
            .set_value(request)
            .await;
        let state = Arc::new(FlowLikeState::new(
            FlowLikeConfig::new(),
            HTTPClient::new_without_refetch(),
        ));
        let run: Weak<Mutex<Run>> = Weak::new();
        ExecutionContext::new(
            Arc::new(AHashMap::from_iter([(
                node.node_id().to_string(),
                node.clone(),
            )])),
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
        .await
    }

    fn cancel_run(context: &mut ExecutionContext) {
        let token = CancellationToken::new();
        token.cancel();
        context.set_cancellation_token(token);
    }

    async fn pin_value(context: &ExecutionContext, name: &str) -> Option<Value> {
        context
            .get_pin_by_name(name)
            .await
            .unwrap()
            .get_raw_value()
            .await
    }

    #[tokio::test]
    async fn dropping_a_node_run_cancels_its_handler() {
        let mut context = boost_context(boost_request(4, 1)).await;
        let (stopped, observed) = mpsc::channel();
        let run = run_cancellable_operation::<Value, (), _>(&mut context, move |_, cancel| {
            let result = wait_for_cancel(&cancel);
            let _ = stopped.send(result.is_err());
            result
        });
        // The executor drops a cancelled run's node future without polling it again.
        assert!(run.now_or_never().is_none());
        assert!(observed.recv_timeout(Duration::from_secs(30)).unwrap());
        assert_eq!(pin_value(&context, "exec_out").await, Some(json!(false)));
        assert_eq!(pin_value(&context, "result").await, None);
    }

    #[tokio::test]
    async fn a_cancelled_run_token_stops_the_handler() {
        let mut context = boost_context(boost_request(4, 1)).await;
        cancel_run(&mut context);
        let error = run_cancellable_operation::<Value, (), _>(&mut context, |_, cancel| {
            wait_for_cancel(&cancel)
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "worker stopped after cancellation");
        assert_eq!(pin_value(&context, "exec_out").await, Some(json!(false)));
        assert_eq!(pin_value(&context, "result").await, None);
    }

    #[tokio::test]
    async fn boosted_tree_node_publishes_its_model() {
        let mut context = boost_context(boost_request(8, 2)).await;
        TrainGradientBoostedTreesNode
            .run(&mut context)
            .await
            .unwrap();
        let model = pin_value(&context, "result").await.unwrap();
        assert_eq!(model["architecture"], "histogram_gradient_boosting");
        assert_eq!(pin_value(&context, "exec_out").await, Some(json!(true)));
    }

    #[tokio::test]
    async fn a_cancelled_run_stops_boosted_tree_training() {
        // Enough estimators that the cancel arrives long before training could finish.
        let mut context = boost_context(boost_request(1_000, 10_000)).await;
        cancel_run(&mut context);
        let error = TrainGradientBoostedTreesNode
            .run(&mut context)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("Gradient boosted tree training was cancelled at step"),
            "{error}"
        );
        assert_eq!(pin_value(&context, "exec_out").await, Some(json!(false)));
    }
}
