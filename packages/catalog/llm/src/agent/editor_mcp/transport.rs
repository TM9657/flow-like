use flow_like_types::{Value, json, tokio_util::sync::CancellationToken};
use futures::future::BoxFuture;
use rmcp::{
    RoleClient,
    service::{RxJsonRpcMessage, TxJsonRpcMessage},
    transport::Transport,
};
use std::{
    collections::HashSet,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle};

pub(super) type Requester =
    Arc<dyn Fn(Value) -> BoxFuture<'static, flow_like_types::Result<Value>> + Send + Sync>;
pub(super) const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const CANCEL_TIMEOUT: Duration = Duration::from_secs(2);
const CANCEL_CLEANUP_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Default)]
struct RequestState {
    pending: HashSet<String>,
    cancelled: HashSet<String>,
}

struct Outbound {
    message: Value,
    original_id: Option<Value>,
    wire_id: Option<String>,
}

pub(crate) struct EditorMcpTransport {
    requests: mpsc::Sender<Outbound>,
    responses: mpsc::Receiver<RxJsonRpcMessage<RoleClient>>,
    cancellation: CancellationToken,
    cancel_requester: Requester,
    namespace: String,
    state: Arc<Mutex<RequestState>>,
    task: Option<JoinHandle<()>>,
}

async fn notify_cancel(requester: &Requester, wire_id: &str) {
    let message = json::json!({"jsonrpc":"2.0", "method":"notifications/cancelled", "params": {
        "requestId": wire_id, "reason": "The agent stopped waiting for this editor request"
    }});
    let _ = tokio::time::timeout(CANCEL_TIMEOUT, requester(message)).await;
}

impl EditorMcpTransport {
    pub(super) fn with_requester(
        requester: Requester,
        cancel_requester: Requester,
        cancellation: CancellationToken,
    ) -> Self {
        let (requests, mut incoming) = mpsc::channel::<Outbound>(8);
        let (outgoing, responses) = mpsc::channel(8);
        let state = Arc::new(Mutex::new(RequestState::default()));
        let worker_state = state.clone();
        let cancel = cancellation.clone();
        let cancel_pending = cancel_requester.clone();
        let task = tokio::spawn(async move {
            loop {
                let Outbound {
                    message,
                    original_id,
                    wire_id,
                } = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    item = incoming.recv() => match item { Some(item) => item, None => break },
                };
                let cancelled = wire_id
                    .as_ref()
                    .is_some_and(|id| worker_state.lock().unwrap().cancelled.remove(id));
                let result = if cancelled {
                    Err(flow_like_types::anyhow!("The editor request was cancelled"))
                } else {
                    let request = requester(message);
                    tokio::pin!(request);
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            if let Some(id) = &wire_id { notify_cancel(&cancel_pending, id).await; }
                            // Let the cancelled device wait remove its channel ticket before dropping it.
                            let _ = tokio::time::timeout(CANCEL_CLEANUP_TIMEOUT, &mut request).await;
                            break;
                        },
                        result = tokio::time::timeout(REQUEST_TIMEOUT, &mut request) => match result {
                            Ok(result) => result,
                            Err(_) => {
                                if let Some(id) = &wire_id { notify_cancel(&cancel_pending, id).await; }
                                Err(flow_like_types::anyhow!("The editor request timed out"))
                            }
                        },
                    }
                };
                if let Some(id) = &wire_id {
                    let mut state = worker_state.lock().unwrap();
                    state.pending.remove(id);
                    state.cancelled.remove(id);
                }
                let response = result.and_then(|mut value| {
                    if value.is_null() && original_id.is_none() {
                        return Ok(None);
                    }
                    if let Some(wire_id) = &wire_id {
                        if value.get("id").and_then(Value::as_str) != Some(wire_id) {
                            return Err(flow_like_types::anyhow!(
                                "The editor returned a response for a different request"
                            ));
                        }
                        value["id"] = original_id.clone().unwrap();
                    }
                    json::from_value::<RxJsonRpcMessage<RoleClient>>(value)
                        .map(Some)
                        .map_err(Into::into)
                });
                let response = match response {
                    Ok(None) => continue,
                    Ok(Some(response)) => response,
                    Err(error) => {
                        let Some(id) = original_id else { break };
                        let error = json::json!({"jsonrpc":"2.0", "id":id, "error":{
                            "code":-32000, "message":error.to_string().chars().take(2048).collect::<String>()
                        }});
                        let Ok(error) = json::from_value(error) else {
                            break;
                        };
                        error
                    }
                };
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    result = outgoing.send(response) => if result.is_err() { break },
                }
            }
        });
        Self {
            requests,
            responses,
            cancellation,
            cancel_requester,
            state,
            namespace: uuid::Uuid::new_v4().to_string(),
            task: Some(task),
        }
    }

    fn wire_id(&self, id: &Value) -> String {
        format!("{}:{}", self.namespace, id)
    }
}

impl Transport<RoleClient> for EditorMcpTransport {
    type Error = io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let message = json::to_value(item)
            .map_err(io::Error::other)
            .map(|mut message| {
                if message["method"] == "notifications/cancelled" {
                    let id = self.wire_id(&message["params"]["requestId"]);
                    message["params"]["requestId"] = Value::String(id.clone());
                    let mut state = self.state.lock().unwrap();
                    if state.pending.contains(&id) {
                        state.cancelled.insert(id);
                    }
                    return (message, None, None, true);
                }
                let original_id = message
                    .get("method")
                    .and_then(|_| message.get("id"))
                    .cloned();
                let wire_id = original_id.as_ref().map(|id| self.wire_id(id));
                if let Some(id) = &wire_id {
                    message["id"] = Value::String(id.clone());
                    self.state.lock().unwrap().pending.insert(id.clone());
                }
                (message, original_id, wire_id, false)
            });
        let sender = self.requests.clone();
        let cancellation = self.cancellation.clone();
        let cancel_requester = self.cancel_requester.clone();
        // Sending only queues work. Waiting for the frontend here can block MCP's receive loop.
        async move {
            let (message, original_id, wire_id, is_cancel) = message?;
            if cancellation.is_cancelled() {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "The editor connection is closed",
                ));
            }
            if is_cancel {
                // Cancellation must pass an in-flight request rather than waiting behind it.
                tokio::spawn(async move {
                    let _ = tokio::time::timeout(CANCEL_TIMEOUT, cancel_requester(message)).await;
                });
                return Ok(());
            }
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => Err(io::Error::new(io::ErrorKind::BrokenPipe, "The editor connection is closed")),
                result = sender.send(Outbound { message, original_id, wire_id }) => result.map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "The editor connection is closed")),
            }
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        tokio::select! {
            biased;
            _ = self.cancellation.cancelled() => None,
            response = self.responses.recv() => response,
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.cancellation.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        Ok(())
    }
}

impl Drop for EditorMcpTransport {
    fn drop(&mut self) {
        self.cancellation.cancel();
        // The worker exits after a bounded channel cleanup, including when rmcp drops a peer.
        self.task.take();
    }
}

#[cfg(test)]
#[path = "transport/tests.rs"]
mod tests;
