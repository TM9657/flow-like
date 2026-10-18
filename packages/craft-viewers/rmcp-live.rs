use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{CallToolResult, ContentBlock as Content};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    future::Future,
    sync::{Arc, mpsc},
    task::{Context, Poll, Waker},
};

#[derive(Debug)]
enum AutomationError {
    Other(String),
    BadRequest(String),
}
impl std::fmt::Display for AutomationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(s) | Self::BadRequest(s) => f.write_str(s),
        }
    }
}
struct ControlCall {
    method: String,
    params: Value,
    reply: mpsc::Sender<Result<Value, String>>,
}
struct BridgeClient {
    tx: mpsc::Sender<ControlCall>,
}
impl BridgeClient {
    async fn call(&self, method: &str, params: Value) -> Result<Value, AutomationError> {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(ControlCall {
                method: method.into(),
                params,
                reply,
            })
            .map_err(|e| AutomationError::Other(e.to_string()))?;
        std::future::poll_fn(move |_| match rx.try_recv() {
            Ok(result) => Poll::Ready(result.map_err(AutomationError::Other)),
            Err(mpsc::TryRecvError::Empty) => Poll::Pending,
            Err(_) => Poll::Ready(Err(AutomationError::Other(
                "The live editor closed.".into(),
            ))),
        })
        .await
    }
    async fn call_read(&self, method: &str, params: Value) -> Result<Value, AutomationError> {
        self.call(method, params).await
    }
    async fn execute(&self, id: &str, params: Value) -> Result<Value, AutomationError> {
        self.call("engine.execute", json!({"command":id,"params":params}))
            .await
    }
}

fn drive<F: Future>(
    future: F,
    rx: &mpsc::Receiver<ControlCall>,
    dispatch: &mut dyn FnMut(&str, Value) -> Result<Value, String>,
) -> Result<F::Output, String> {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..2048 {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return Ok(value);
        }
        let call = rx
            .try_recv()
            .map_err(|_| "This tool needs an asynchronous native operation.".to_string())?;
        let _ = call.reply.send(dispatch(&call.method, call.params));
    }
    Err("The tool exceeded the live editor operation limit.".into())
}
