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
        if let Some(cache) = context.cache.read().await.get(DOCLING_KEY)
            && let Some(docling) = cache.as_any().downcast_ref::<DoclingCacheObject>()
        {
            port = docling.port;
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
        let dir = dirs_next::data_dir()
            .unwrap_or_default()
            .join("flow-like")
            .join("bits")
            .join("docling_cache");
        let dir = dir.to_str().unwrap_or_default();

        let args = vec!["localhost", &port, dir];
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
