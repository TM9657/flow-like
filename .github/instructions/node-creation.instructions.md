---
applyTo: "packages/catalog/**/*.rs"
---
# Node Creation Guidelines

Apply the [general coding guidelines](./general-coding.instructions.md) to all code.

This document describes the creation process of nodes for Flow-Like. Nodes generally have the run functionality and Pins which hold
data.

## Node Creation
Nodes can be Pure and Impure. Pure nodes are those that do not have any side effects and always return the same output for the same input.
Impure nodes are those that may have side effects or return different outputs for the same input.

You will always need Execution Pins on Impure Nodes. No Execution Pins on Pure ones.
Nodes have a `get_node` function that is called from the Node Catalog. It constructs the structure of a node. The `run` function is called when the node is executed.
Here you typically first deactivate outgoing execution pins (in case of failure, this stops the graph execution), next you will execute the logic of the node and finally you will activate the outgoing execution pins and write the data into the pins. Pins hold serialized Serde Values.

Optionally you can add an on_update function that runs on every board parse event and allows to dynamically adjust the pins (for example if a specific selection triggers new pins options, or if you can regex parse a node and extract optional pins that are necessary).

If you need more abstract memory, like a thread-handle or database connections you can use the contexts cache with Any.

## Tipps and Tricks
- Log out warnings, errors etc.
- Multiple Pins with the same name are allowed, they will offer the user to add more pins of this same type to the node.
- Set the Options to offer the user enum drop downs, set a schema for struct pins, it is super helpful.
- If you can, set default values.
- You can abstract inputs using JsonSchemar Structs (and use their schema in the Pin Options) to created typed interactions.
- Success and Error pins should be offered when we leave the workflow system (e.g API calls, Database Connections, etc.). The workflow offers an explicit way to handle errors and success cases out of the box, which can be used in the other cases.

## Types to Use
- for Images use: NodeImage
- for Files use: FlowPath

## Example Nodes
<Example Pure Node>
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct BoolOr {}

impl BoolOr {
    pub fn new() -> Self {
        BoolOr {}
    }
}

#[async_trait]
impl NodeLogic for BoolOr {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("bool_or", "Or", "Boolean Or operation", "Utils/Bool");
        node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for OR Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for OR Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "result",
            "Result",
            "OR operation between all boolean inputs",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut output_value = false;

        let boolean_pins = context.get_pins_by_name("boolean").await?;

        for pin in boolean_pins {
            let pin = context.evaluate_pin_ref(pin).await?;

            output_value = output_value || pin;
            if output_value {
                break;
            }
        }

        let result = context.get_pin_by_name("result").await?;

        context.log_message(
            &format!("OR Operation Result: {}", output_value),
            LogLevel::Debug,
        );
        context
            .set_pin_ref_value(&result, json!(output_value))
            .await?;

        return Ok(());
    }
}
</Example Pure Node>

<Example Simple Impure Node>
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::async_trait;

#[derive(Default)]
pub struct BranchNode {}

impl BranchNode {
    pub fn new() -> Self {
        BranchNode {}
    }
}

#[async_trait]
impl NodeLogic for BranchNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_branch",
            "Branch",
            "Branches the flow based on a condition",
            "Control",
        );
        node.add_icon("/flow/icons/split.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "condition",
            "Condition",
            "The condition to evaluate",
            VariableType::Boolean,
        )
        .set_default_value(Some(flow_like_types::json::json!(true)));

        node.add_output_pin(
            "true",
            "True",
            "The flow to follow if the condition is true",
            VariableType::Execution,
        );
        node.add_output_pin(
            "false",
            "False",
            "The flow to follow if the condition is false",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let condition = context.evaluate_pin::<bool>("condition").await?;

        let true_pin = context.get_pin_by_name("true").await?;
        let false_pin = context.get_pin_by_name("false").await?;

        context.deactivate_exec_pin_ref(&true_pin).await?;
        context.deactivate_exec_pin_ref(&false_pin).await?;

        if condition {
            context.activate_exec_pin_ref(&true_pin).await?;
            context.deactivate_exec_pin_ref(&false_pin).await?;

            return Ok(());
        }

        context.deactivate_exec_pin_ref(&true_pin).await?;
        context.activate_exec_pin_ref(&false_pin).await?;

        return Ok(());
    }
}
</Example Simple Impure Node>

<Example Complex Impure Node>
use crate::storage::path::FlowPath;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::Path;
use flow_like_types::{
    Cacheable, async_trait,
    json::json,
    reqwest,
    tokio::{self, task::JoinHandle, time::sleep},
};
use std::{
    any::Any,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Child,
    sync::{Arc, Mutex},
    time::Duration,
};

static DOCLING_KEY: &str = "docling_process";

pub struct DoclingCacheObject {
    pub port: u16,
    handle: Arc<Mutex<Option<Child>>>,
    thread_handle: JoinHandle<()>,
}

impl Cacheable for DoclingCacheObject {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Drop for DoclingCacheObject {
    fn drop(&mut self) {
        println!("DROPPING DOCLING THREAD");
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(child) = guard.as_mut() {
                match child.kill() {
                    Ok(_) => println!("Child process was killed successfully."),
                    Err(e) => eprintln!("Failed to kill child process: {}", e),
                }
            } else {
                eprintln!("No docling child process to kill.");
            }
        } else {
            eprintln!("Failed to lock docling handle for dropping.");
        }

        self.thread_handle.abort();
    }
}

#[derive(Default)]
pub struct DoclingNode {}

impl DoclingNode {
    pub fn new() -> Self {
        DoclingNode {}
    }
}

#[async_trait]
impl NodeLogic for DoclingNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_processing_docling",
            "Docling Parse",
            "Parses the input text using Docling",
            "AI/Processing",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_input_pin("file", "File", "The file to process", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "image_export_mode",
            "Image Export Mode",
            "How to handle images (EMBEDDED, REFERENCED, etc.)",
            VariableType::String,
        )
        .set_default_value(Some(json!("embedded")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "embedded".to_string(),
                    "placeholder".to_string(),
                    "referenced".to_string(),
                ])
                .build(),
        );

        node.add_input_pin(
            "force_ocr",
            "Force OCR",
            "Force OCR processing",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "ocr_lang",
            "OCR Language",
            "Language for OCR processing",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "pdf_backend",
            "PDF Backend",
            "PDF processing backend",
            VariableType::String,
        )
        .set_default_value(Some(json!("dlparse_v4")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "dlparse_v4".to_string(),
                    "dlparse_v2".to_string(),
                    "dlparse_v1".to_string(),
                    "pypdfium2".to_string(),
                ])
                .build(),
        );

        node.add_input_pin(
            "table_mode",
            "Table Mode",
            "Enable table processing",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "The parsed output",
            VariableType::Execution,
        );

        node.add_output_pin(
            "markdown",
            "Markdown",
            "The parsed output",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let mut port = 0;
        if let Some(cache) = context.cache.read().await.get(DOCLING_KEY) {
            if let Some(docling) = cache.as_any().downcast_ref::<DoclingCacheObject>() {
                port = docling.port;
            }
        };

        if port == 0 {
            port = start_docling_server(context).await?;
        }

        let file: FlowPath = context.evaluate_pin("file").await?;
        let image_export_mode: String = context.evaluate_pin("image_export_mode").await?;
        let force_ocr: bool = context.evaluate_pin("force_ocr").await?;
        let ocr_lang: String = context.evaluate_pin("ocr_lang").await?;
        let pdf_backend: String = context.evaluate_pin("pdf_backend").await?;
        let table_mode: bool = context.evaluate_pin("table_mode").await?;

        let file_name = Path::from(file.path.clone());
        let file_name = file_name.filename();
        let file_name = file_name.unwrap_or_default().to_string();
        let file_buffer = file.get(context, false).await?;

        let client = reqwest::Client::new();
        let mut form = reqwest::multipart::Form::new()
            .text("image_export_mode", image_export_mode)
            .text("pdf_backend", pdf_backend);

        if !ocr_lang.is_empty() {
            form = form.text("ocr_lang", ocr_lang);
        }

        if table_mode {
            form = form.text("table_mode", "true");
        }

        if force_ocr {
            form = form.text("force_ocr", "true");
        }

        form = form.part(
            "file",
            reqwest::multipart::Part::bytes(file_buffer).file_name(file_name.clone()),
        );
        let url = format!("http://localhost:{}/convert", port);
        let response = client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to send request: {}", e))?;
        if !response.status().is_success() {
            return Err(flow_like_types::anyhow!(
                "Docling request failed with status: {}, {}",
                response.status(),
                file_name
            ));
        }
        let response_text = response
            .text()
            .await
            .map_err(|e| flow_like_types::anyhow!("Failed to read response text: {}", e))?;

        context
            .set_pin_value("markdown", json!(response_text))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        return Ok(());
    }
}

async fn health_check(port: u16) -> flow_like_types::Result<bool> {
    let response = reqwest::get(format!("http://localhost:{}/health", port)).await?;
    if response.status().is_success() {
        Ok(true)
    } else {
        Err(flow_like_types::anyhow!(
            "Docling server is not healthy: {}",
            response.status()
        ))
    }
}

pub async fn start_docling_server(context: &mut ExecutionContext) -> flow_like_types::Result<u16> {
    let port = flow_like::utils::portpicker::pick_unused_port()
        .ok_or_else(|| flow_like_types::anyhow!("No available port found for Docling server"))?;

    let child_handle = Arc::new(Mutex::new(None));
    let child_handle_clone = Arc::clone(&child_handle);

    let thread_handle = tokio::task::spawn(async move {
        let program = PathBuf::from("flow-docling");
        let mut sidecar = match flow_like::utils::execute::sidecar(&program, None).await {
            Ok(sidecar) => sidecar,
            Err(e) => {
                println!("Error: {}", e);
                return;
            }
        };

        let port = port.to_string();
        let args = vec!["localhost", &port, "./docling_cache"];
        println!("Starting Docling Server with args: {:?}", args);

        let mut child = sidecar
            .args(args)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn sidecar");

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        *child_handle_clone.lock().unwrap() = Some(child);

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        tokio::spawn(async move {
            stdout_lines.by_ref().flatten().for_each(|line| {
                println!("[DOCLING] stdout: {}", line);
            });
        });

        tokio::spawn(async move {
            stderr_lines.by_ref().flatten().for_each(|line| {
                eprintln!("[DOCLING ERROR] stderr: {}", line);
            });
        });
    });

    let mut loaded = false;
    let mut max_retries = 60;

    while !loaded && max_retries > 0 {
        match health_check(port).await {
            Ok(_) => loaded = true,
            Err(_e) => {
                sleep(Duration::from_secs(1)).await;
                max_retries -= 1;
            }
        }
    }

    let docling_cache = DoclingCacheObject {
        port,
        handle: child_handle,
        thread_handle,
    };
    let cacheable: Arc<dyn Cacheable> = Arc::new(docling_cache);
    context
        .cache
        .write()
        .await
        .insert(DOCLING_KEY.to_string(), cacheable);

    Ok(port)
}
</Example Complex Impure Node>

<Example Node with on_update Function>
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext, internal_node::InternalNode},
        node::{Node, NodeLogic},
        pin::PinType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Value, async_trait, json::from_slice};

#[derive(Default)]
pub struct CallReferenceNode {}

impl CallReferenceNode {
    pub fn new() -> Self {
        CallReferenceNode {}
    }
}

#[async_trait]
impl NodeLogic for CallReferenceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_call_reference",
            "Call Reference",
            "References a specific call in the flow",
            "Control/Call",
        );
        node.add_icon("/flow/icons/workflow.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "fn_ref",
            "Function Reference",
            "The function reference to call",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Done",
            "The flow to follow if the function call is successful",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let fn_ref: String = context.evaluate_pin("fn_ref").await?;

        let mut content_pins = HashMap::with_capacity(context.node.pins.len());
        let input_pins = context.node.pins.clone();

        for (_id, pin) in input_pins {
            let value = context.evaluate_pin_ref::<Value>(pin.clone()).await?;
            let name = pin.lock().await.pin.lock().await.name.clone();
            content_pins.insert(name, value);
        }

        let reference_function = context
            .nodes
            .get(&fn_ref)
            .ok_or_else(|| flow_like_types::anyhow!("Function reference not found"))?;

        let node_ref = reference_function.node.clone();

        let pins = reference_function.pins.clone();
        for (_id, pin) in pins {
            let guard = pin.lock().await;
            let (pin_type, data_type, name) = {
                let pin = guard.pin.lock().await;
                (
                    pin.pin_type.clone(),
                    pin.data_type.clone(),
                    pin.name.clone(),
                )
            };
            if pin_type == PinType::Input || data_type == VariableType::Execution {
                continue;
            }

            if let Some(value) = content_pins.get(&name) {
                guard.set_value(value.clone()).await;
            }
        }

        let mut sub_context = context.create_sub_context(reference_function).await;
        sub_context.delegated = true;
        let run = InternalNode::trigger(&mut sub_context, &mut None, true).await;
        sub_context.end_trace();
        context.push_sub_context(sub_context);

        if run.is_err() {
            let node_name = node_ref.lock().await.friendly_name.clone();
            let error = run.err().unwrap();
            context.log_message(
                &format!("Error: {:?} calling function {}", error, node_name),
                LogLevel::Error,
            );
        }

        context.activate_exec_pin("exec_out").await?;
        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        node.error = None;
        let node_ref = match node.get_pin_by_name("fn_ref") {
            Some(pin) => pin.clone(),
            None => {
                node.error = Some("Function reference pin not found".to_string());
                return;
            }
        };

        let reference = match node_ref.default_value {
            Some(value) => value,
            None => {
                node.error = Some("Function reference pin is not connected".to_string());
                return;
            }
        };

        let reference = match from_slice::<String>(&reference) {
            Ok(value) => value,
            Err(err) => {
                node.error = Some(format!("Failed to parse function reference: {}", err));
                return;
            }
        };

        let event = match board.nodes.get(&reference) {
            Some(event) => event.clone(),
            None => {
                node.error = Some(format!("Function reference not found: {}", reference));
                return;
            }
        };

        node.friendly_name = format!("Call {}", event.friendly_name);
        node.description = format!("Calls the function {}", event.friendly_name);
        node.icon = event.icon.clone();

        let mut output_pins = event
            .pins
            .iter()
            .filter(|pin| {
                pin.1.pin_type == PinType::Output && pin.1.data_type != VariableType::Execution
            })
            .map(|pin| {
                let mut pin = pin.1.clone();
                pin.index += 1;
                pin
            })
            .collect::<Vec<_>>();

        output_pins.sort_by(|a, b| a.index.cmp(&b.index));
        let mut relevant_pins = HashSet::with_capacity(output_pins.len());
        for pin in output_pins {
            relevant_pins.insert(pin.name.clone());
            if node.pins.iter().any(|(_, p)| p.name == pin.name) {
                continue;
            }
            let new_pin = node.add_input_pin(
                &pin.name,
                &pin.friendly_name,
                &pin.description,
                pin.data_type,
            );
            new_pin.schema = pin.schema.clone();
            new_pin.options = pin.options.clone();
        }
        node.pins.retain(|_, pin| {
            if pin.pin_type == PinType::Input && pin.data_type != VariableType::Execution {
                relevant_pins.contains(&pin.name) || pin.name == "fn_ref"
            } else {
                true
            }
        });
    }
}
</Example Node with on_update Function>