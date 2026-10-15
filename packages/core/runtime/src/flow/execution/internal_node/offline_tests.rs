use super::*;
use crate::{
    flow::{
        board::{Board, ExecutionStage, Layer, LayerType},
        compiled::{compile::compile_board, template::CompiledRunTemplate},
        execution::{ExecutionEnvironment, InternalRun, RunPayload},
    },
    profile::Profile,
    state::{FlowLikeConfig, FlowLikeState, FlowNodeRegistryInner},
    utils::http::HTTPClient,
};
use flow_like_storage::Path;
use flow_like_types::{async_trait, sync::RwLock, tokio};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingLogic {
    schema: Node,
    calls: AtomicUsize,
}

impl CountingLogic {
    fn new(name: &str, only_offline: bool, pure: bool) -> Arc<Self> {
        let mut schema = Node::new(name, name, "Execution policy test", "Tests");
        schema.set_only_offline(only_offline);
        if !pure {
            schema.add_input_pin("exec_in", "Run", "Run", VariableType::Execution);
            schema.add_output_pin("exec_out", "Continue", "Continue", VariableType::Execution);
            schema.add_input_pin("value", "Value", "Dependency", VariableType::Integer);
        }
        schema.add_output_pin("value_out", "Value", "Result", VariableType::Integer);
        Arc::new(Self {
            schema,
            calls: AtomicUsize::new(0),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl NodeLogic for CountingLogic {
    fn get_node(&self) -> Node {
        self.schema.clone()
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        context.set_pin_value("value_out", json!(1)).await?;
        if !self.schema.is_pure() {
            context.activate_exec_pin("exec_out").await?;
        }
        Ok(())
    }
}

fn internal(logic: &Arc<CountingLogic>) -> Arc<InternalNode> {
    let mut placed = logic.get_node();
    // A workflow author can alter serialized node metadata.
    placed.only_offline = false;
    let pins = placed
        .pins
        .values()
        .map(|pin| (pin.id.clone(), Arc::new(InternalPin::new(pin, false))))
        .collect();
    let node = Arc::new(InternalNode::new(
        placed,
        pins,
        logic.clone(),
        AHashMap::new(),
    ));
    for pin in &node.pins {
        pin.init_node(Arc::downgrade(&node));
    }
    node
}

fn state(environment: ExecutionEnvironment) -> Arc<FlowLikeState> {
    let mut state = FlowLikeState::new(FlowLikeConfig::new(), HTTPClient::new_without_refetch());
    state.execution_environment = environment;
    Arc::new(state)
}

async fn context(
    environment: ExecutionEnvironment,
    root: &Arc<InternalNode>,
    nodes: &[Arc<InternalNode>],
) -> ExecutionContext {
    ExecutionContext::new(
        Arc::new(
            nodes
                .iter()
                .map(|node| (node.node_id().to_owned(), node.clone()))
                .collect(),
        ),
        &Weak::new(),
        &state(environment),
        root,
        &Arc::new(Mutex::new(AHashMap::new())),
        &Arc::new(RwLock::new(AHashMap::new())),
        LogLevel::Debug,
        ExecutionStage::Dev,
        Arc::new(Profile::default()),
        None,
        Arc::new(RwLock::new(vec![])),
        None,
        None,
        Arc::new(AHashMap::new()),
        None,
    )
    .await
}

async fn trigger(
    context: &mut ExecutionContext,
    mapped: bool,
    successors: bool,
    dependencies: &AHashMap<String, Vec<Arc<InternalNode>>>,
) -> Result<(), InternalNodeError> {
    if mapped {
        InternalNode::trigger_with_dependencies(context, &mut None, successors, dependencies).await
    } else {
        InternalNode::trigger(context, &mut None, successors).await
    }
}

async fn connect(
    source: &Arc<InternalNode>,
    output: &str,
    target: &Arc<InternalNode>,
    input: &str,
) {
    let output = source.get_pin_by_name(output).await.unwrap();
    let input = target.get_pin_by_name(input).await.unwrap();
    output.init_connected_to(vec![Arc::downgrade(&input)]);
    input.init_depends_on(vec![Arc::downgrade(&output)]);
}

#[tokio::test]
async fn server_rejects_offline_logic_despite_forged_placed_metadata() {
    for mapped in [false, true] {
        let logic = CountingLogic::new("offline_action", true, false);
        let node = internal(&logic);
        let mut context = context(ExecutionEnvironment::Server, &node, &[node.clone()]).await;

        let error = trigger(&mut context, mapped, false, &AHashMap::new())
            .await
            .expect_err("server dispatch must refuse the registered offline node")
            .into_message();

        assert!(error.contains("offline_action"), "{error}");
        assert!(error.contains("requires local execution"), "{error}");
        assert_eq!(logic.calls(), 0, "mapped={mapped}");
        assert_eq!(context.get_state(), NodeState::Error);
    }
}

#[tokio::test]
async fn public_parts_constructor_preserves_the_logic_restriction() {
    let logic = CountingLogic::new("offline_parts", true, false);
    let source = internal(&logic);
    let node = Arc::new(InternalNode::from_parts(
        source.node.clone(),
        source.meta.clone(),
        source.pins.clone(),
        source.lookup.clone(),
        source.logic.clone(),
    ));
    let mut context = context(ExecutionEnvironment::Server, &node, &[node.clone()]).await;
    assert!(
        trigger(&mut context, false, false, &AHashMap::new())
            .await
            .is_err()
    );
    assert_eq!(logic.calls(), 0);
}

#[tokio::test]
async fn local_environments_preserve_offline_node_execution() {
    for environment in [
        ExecutionEnvironment::Local,
        ExecutionEnvironment::Desktop,
        ExecutionEnvironment::Mobile,
        ExecutionEnvironment::BrowserSandbox,
    ] {
        for mapped in [false, true] {
            let logic = CountingLogic::new("offline_action", true, false);
            let node = internal(&logic);
            let mut context = context(environment, &node, &[node.clone()]).await;
            trigger(&mut context, mapped, false, &AHashMap::new())
                .await
                .unwrap();
            assert_eq!(logic.calls(), 1, "{environment:?}, mapped={mapped}");
        }
    }
}

#[tokio::test]
async fn server_preserves_unrestricted_node_execution() {
    for mapped in [false, true] {
        let logic = CountingLogic::new("portable_action", false, false);
        let node = internal(&logic);
        let mut context = context(ExecutionEnvironment::Server, &node, &[node.clone()]).await;
        trigger(&mut context, mapped, false, &AHashMap::new())
            .await
            .unwrap();
        assert_eq!(logic.calls(), 1, "mapped={mapped}");
    }
}

#[tokio::test]
async fn offline_pure_dependencies_cannot_run_through_a_portable_node() {
    for environment in [ExecutionEnvironment::Server, ExecutionEnvironment::Local] {
        for mapped in [false, true] {
            let dependency_logic = CountingLogic::new("offline_value", true, true);
            let consumer_logic = CountingLogic::new("portable_consumer", false, false);
            let dependency = internal(&dependency_logic);
            let consumer = internal(&consumer_logic);
            connect(&dependency, "value_out", &consumer, "value").await;
            let dependencies =
                AHashMap::from([(consumer.node_id().to_owned(), vec![dependency.clone()])]);
            let mut context =
                context(environment, &consumer, &[consumer.clone(), dependency]).await;

            let result = trigger(&mut context, mapped, false, &dependencies).await;
            if environment == ExecutionEnvironment::Server {
                assert!(result.is_err(), "mapped={mapped}");
                assert_eq!(dependency_logic.calls(), 0, "mapped={mapped}");
                assert_eq!(consumer_logic.calls(), 0, "mapped={mapped}");
            } else {
                result.unwrap();
                assert_eq!(dependency_logic.calls(), 1, "mapped={mapped}");
                assert_eq!(consumer_logic.calls(), 1, "mapped={mapped}");
            }
        }
    }
}

#[tokio::test]
async fn offline_successors_cannot_run_through_a_portable_entry() {
    for environment in [ExecutionEnvironment::Server, ExecutionEnvironment::Local] {
        for mapped in [false, true] {
            let entry_logic = CountingLogic::new("portable_entry", false, false);
            let successor_logic = CountingLogic::new("offline_successor", true, false);
            let entry = internal(&entry_logic);
            let successor = internal(&successor_logic);
            connect(&entry, "exec_out", &successor, "exec_in").await;
            let mut context = context(environment, &entry, &[entry.clone(), successor]).await;

            let result = trigger(&mut context, mapped, true, &AHashMap::new()).await;
            assert_eq!(entry_logic.calls(), 1, "{environment:?}, mapped={mapped}");
            if environment == ExecutionEnvironment::Server {
                assert!(result.is_err(), "mapped={mapped}");
                assert_eq!(successor_logic.calls(), 0, "mapped={mapped}");
            } else {
                result.unwrap();
                assert_eq!(successor_logic.calls(), 1, "mapped={mapped}");
            }
        }
    }
}

#[tokio::test]
async fn compiled_templates_use_catalog_restrictions_for_top_level_and_function_nodes() {
    // Both legacy function bodies and current layer-tagged nodes must be checked.
    for placement in ["top_level", "function_body", "layer_tag"] {
        let logic = CountingLogic::new("offline_compiled_action", true, false);
        let mut registry = FlowNodeRegistryInner::new(1);
        registry.insert(logic.get_node(), logic.clone());
        let mut board = Board::new_detached(Some("offline-policy".into()), Path::default());
        let mut node = logic.get_node();
        node.only_offline = false;
        let node_id = node.id.clone();
        if placement == "top_level" {
            board.nodes.insert(node.id.clone(), node);
        } else {
            let mut layer = Layer::new("function".into(), "Function".into(), LayerType::Function);
            if placement == "function_body" {
                layer.nodes.insert(node.id.clone(), node);
            } else {
                node.layer = Some(layer.id.clone());
                board.nodes.insert(node.id.clone(), node);
            }
            board.layers.insert(layer.id.clone(), layer);
        }
        let compiled = compile_board(&board).unwrap();
        assert!(compiled.nodes.iter().all(|node| !node.only_offline));
        let template = Arc::new(
            CompiledRunTemplate::from_compiled(&compiled, &registry, Path::default()).unwrap(),
        );

        let error = template
            .ensure_execution_environment(ExecutionEnvironment::Server)
            .expect_err("compiled offline metadata must not override the catalog");
        assert!(error.to_string().contains("offline_compiled_action"));
        template
            .ensure_execution_environment(ExecutionEnvironment::Local)
            .unwrap();

        let result = InternalRun::from_template(
            "test-app",
            template,
            None,
            &state(ExecutionEnvironment::Server),
            &Profile::default(),
            &RunPayload {
                id: node_id,
                payload: None,
                runtime_variables: None,
                filter_secrets: Some(true),
            },
            false,
            None,
            None,
            None,
            Default::default(),
            None,
            None,
        )
        .await;
        assert!(result.is_err(), "placement={placement}");
        assert_eq!(logic.calls(), 0, "placement={placement}");
    }
}
