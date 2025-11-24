use crate::{
    bit::{Bit, BitTypes},
    models::{ModelMeta, local_utils::ensure_local_weights},
    state::FlowLikeState,
};
use flow_like_model_provider::llm::{ModelLogic, llamacpp::LlamaCppModel};
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_types::{
    Result, reqwest,
    tokio::{self, sync::Mutex as TokioMutex, task::JoinHandle, time::sleep},
};
use portpicker::pick_unused_port;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Child,
    sync::{Arc, Mutex},
    time::Duration,
};

use super::ExecutionSettings;

pub struct LocalModel {
    bit: Bit,
    handle: Arc<Mutex<Option<Child>>>,
    thread_handle: JoinHandle<()>,
    llm_model: Arc<LlamaCppModel>,
    pub port: u16,
}

impl ModelMeta for LocalModel {
    fn get_bit(&self) -> Bit {
        self.bit.clone()
    }
}

#[flow_like_types::async_trait]
impl ModelLogic for LocalModel {
    async fn provider(&self) -> Result<flow_like_model_provider::llm::ModelConstructor> {
        self.llm_model.provider().await
    }

    async fn default_model(&self) -> Option<String> {
        self.llm_model.default_model().await
    }
}

impl LocalModel {
    pub async fn check_health(port: &str) -> Result<bool> {
        let response = reqwest::get(format!("http://localhost:{}/health", port)).await?;

        if response.status().is_success() {
            Ok(true)
        } else {
            Err(flow_like_types::anyhow!(
                "Model is not healthy: {}",
                response.status()
            ))
        }
    }

    pub async fn new(
        bit: &Bit,
        app_state: Arc<TokioMutex<FlowLikeState>>,
        execution_settings: &ExecutionSettings,
    ) -> flow_like_types::Result<LocalModel> {
        let bit_store = FlowLikeState::bit_store(&app_state).await?;

        let bit_store = match bit_store {
            FlowLikeStore::Local(store) => store,
            _ => return Err(flow_like_types::anyhow!("Only local store supported")),
        };

        let gguf_path = bit
            .to_path(&bit_store)
            .ok_or(flow_like_types::anyhow!("No model path"))?;
        let pack = bit.pack(app_state.clone()).await?;
        ensure_local_weights(&pack, &app_state, bit.id.as_str(), "local model").await?;

        let projection_bit = pack
            .bits
            .iter()
            .find(|b| b.bit_type == BitTypes::Projection);
        let projection_bit = projection_bit.cloned();

        let child_handle = Arc::new(Mutex::new(None));
        let child_handle_clone: Arc<Mutex<Option<Child>>> = Arc::clone(&child_handle);
        let port = pick_unused_port().unwrap();

        let async_bit = bit.clone();
        let execution_settings = execution_settings.clone();
        let thread_handle = tokio::task::spawn(async move {
            let program = PathBuf::from("llama-server");
            let mut sidecar = match crate::utils::execute::sidecar(&program, None).await {
                Ok(sidecar) => sidecar,
                Err(e) => {
                    println!("Error: {}", e);
                    return;
                }
            };
            let mut context_length = async_bit.try_to_context_length().unwrap_or(512);
            context_length =
                std::cmp::min(context_length, execution_settings.max_context_size as u32);
            let binding = context_length.to_string();
            let port = port.to_string();
            println!("Execution settings: {:?}", execution_settings);
            let mut args = vec![
                "-m",
                &gguf_path.to_str().unwrap(),
                "-c",
                &binding,
                "--host",
                "localhost",
                "--port",
                &port,
                "--no-webui",
                "--jinja",
            ];

            let mut gpu_layer = 0;

            if execution_settings.gpu_mode {
                gpu_layer = 45;
            }

            let gpu_layer = gpu_layer.to_string();
            args.push("-ngl");
            args.push(&gpu_layer);

            println!("Starting LLM Server with args: {:?}", args);

            let mut projection_path = String::new();
            if let Some(projection_bit) = projection_bit {
                let path = projection_bit.to_path(&bit_store);
                if let Some(path) = path {
                    projection_path = path.to_str().unwrap().to_string();
                }
            }

            if !projection_path.is_empty() {
                args.push("--mmproj");
                args.push(&projection_path);
            }

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
                    println!("[LLM] stdout: {}", line);
                });
            });

            tokio::spawn(async move {
                stderr_lines.by_ref().flatten().for_each(|line| {
                    eprintln!("[LLM ERROR] stderr: {}", line);
                });
            });
        });

        let mut loaded = false;
        let mut max_retries = 60;

        while !loaded && max_retries > 0 {
            match LocalModel::check_health(&port.to_string()).await {
                Ok(_) => loaded = true,
                Err(_e) => {
                    sleep(Duration::from_secs(1)).await;
                    max_retries -= 1;
                }
            }
        }

        if !loaded {
            return Err(flow_like_types::anyhow!(
                "Failed to start local model server"
            ));
        }

        let provider = bit
            .try_to_provider()
            .ok_or_else(|| flow_like_types::anyhow!("Failed to get provider from bit"))?;

        let llm_model = LlamaCppModel::new(&provider, port).await?;

        Ok(LocalModel {
            bit: bit.clone(),
            handle: child_handle,
            thread_handle,
            llm_model: Arc::new(llm_model),
            port,
        })
    }
}

impl Drop for LocalModel {
    fn drop(&mut self) {
        println!("DROPPING LOCAL MODEL");
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(child) = guard.as_mut() {
                match child.kill() {
                    Ok(_) => println!("Child process was killed successfully."),
                    Err(e) => eprintln!("Failed to kill child process: {}", e),
                }
            } else {
                println!("No child process to kill.");
            }
        } else {
            println!("Failed to lock local model handle for dropping.");
        }

        self.thread_handle.abort();
    }
}
