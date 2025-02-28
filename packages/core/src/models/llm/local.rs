use crate::{
    bit::{Bit, BitTypes},
    models::{
        history::History, llm::ModelLogic, response::Response, response_chunk::ResponseChunk,
    },
    state::{FlowLikeState, FlowLikeStore},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use portpicker::pick_unused_port;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Child,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::Mutex as TokioMutex;
use tokio::time::sleep;

use super::{ExecutionSettings, LLMCallback};

mod local_history;

pub struct LocalModel {
    bit: Bit,
    handle: Arc<Mutex<Option<Child>>>,
    thread_handle: tokio::task::JoinHandle<()>,
    client: reqwest::Client,
    pub port: u16,
}

#[async_trait]
impl ModelLogic for LocalModel {
    fn get_bit(&self) -> Bit {
        self.bit.clone()
    }

    async fn invoke(&self, history: &History, callback: Option<LLMCallback>) -> Result<Response> {
        let local_history = local_history::LocalModelHistory::from_history(history).await;
        let stream = history.stream.unwrap_or(false);

        let request = self
            .client
            .post(format!(
                "http://localhost:{}/v1/chat/completions",
                self.port
            ))
            .json(&local_history);

        let response = request.send().await?;

        if !stream {
            let response = response.json::<Response>().await?;
            return Ok(response);
        }

        let mut output = Response::default();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let chunk = &chunk[6..];
                    match serde_json::from_slice::<ResponseChunk>(chunk) {
                        Ok(parsed_chunk) => {
                            if let Some(callback) = &callback {
                                callback(parsed_chunk.clone()).await?;
                            }
                            output.push_chunk(parsed_chunk);
                        }
                        Err(_) => println!("Failed to parse chunk."),
                    }
                }
                Err(_) => println!("Failed to get chunk."),
            }
        }

        Ok(output)
    }
}

impl LocalModel {
    pub async fn new(
        bit: &Bit,
        app_state: Arc<TokioMutex<FlowLikeState>>,
        execution_settings: &ExecutionSettings,
    ) -> anyhow::Result<LocalModel> {
        let bit_store = FlowLikeState::bit_store(&app_state).await?;

        let bit_store = match bit_store {
            FlowLikeStore::Local(store) => store,
            _ => return Err(anyhow::anyhow!("Only local store supported")),
        };

        let gguf_path = bit
            .to_path(&bit_store)
            .ok_or(anyhow::anyhow!("No model path"))?;
        let pack = bit.pack(app_state.clone()).await?;
        pack.download(app_state).await?;

        let projection_bit = pack
            .bits
            .iter()
            .find(|b| b.bit_type == BitTypes::Projection);
        let projection_bit = projection_bit.cloned();
        let mut current_dir = std::env::current_exe().unwrap();
        current_dir.pop();

        let child_handle = Arc::new(Mutex::new(None));
        let child_handle_clone = Arc::clone(&child_handle);
        let port = pick_unused_port().unwrap();

        let async_bit = bit.clone();
        let execution_settings = execution_settings.clone();
        let thread_handle = tokio::task::spawn(async move {
            let program = PathBuf::from("llamafiler");
            let mut sidecar = match crate::utils::execute::sidecar(&program).await {
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
            let port = format!("localhost:{}", port);
            let mut args = vec![
                "-m",
                &gguf_path.to_str().unwrap(),
                "-c",
                &binding,
                "-l",
                &port,
            ];

            let mut gpu_layer = 0;

            if execution_settings.gpu_mode {
                gpu_layer = 9999;
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
                args.push("-mm");
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

        sleep(Duration::from_millis(2000)).await;

        Ok(LocalModel {
            client: reqwest::Client::new(),
            bit: bit.clone(),
            handle: child_handle,
            thread_handle,
            port,
        })
    }
}

impl Drop for LocalModel {
    fn drop(&mut self) {
        println!("DROPPING LOCAL MODEL");
        let mut guard = self.handle.lock().unwrap();
        if let Some(child) = guard.as_mut() {
            match child.kill() {
                Ok(_) => println!("Child process was killed successfully."),
                Err(e) => eprintln!("Failed to kill child process: {}", e),
            }
        } else {
            println!("No child process to kill.");
        }

        self.thread_handle.abort();
    }
}
