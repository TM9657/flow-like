use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use cuid2::create_id;
use dashmap::DashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, to_value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InterComEvent {
    pub event_id: String,
    pub event_type: String,
    pub payload: Value,
    pub timestamp: SystemTime,
}

pub type InterComCallback = Option<
    Arc<
        dyn Fn(InterComEvent) -> futures::future::BoxFuture<'static, anyhow::Result<()>>
            + Send
            + Sync,
    >,
>;
pub type BatchedCallback = Arc<
    dyn Fn(Vec<InterComEvent>) -> futures::future::BoxFuture<'static, anyhow::Result<()>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct BufferedInterComHandler {
    callback: BatchedCallback,
    interval_ms: u64,
    capacity: u64,
    buffer: Arc<DashMap<String, Vec<InterComEvent>>>,
    last_tick_ms: Arc<AtomicU64>,
}

impl BufferedInterComHandler {
    pub fn new(callback: BatchedCallback, interval_ms: Option<u64>, capacity: Option<u64>) -> Self {
        let interval_ms = interval_ms.unwrap_or(20);
        let capacity = capacity.unwrap_or(200);
        let last_tick_ms = Arc::new(AtomicU64::new(0));

        Self {
            buffer: Arc::new(DashMap::with_capacity(capacity as usize)),
            callback,
            interval_ms,
            capacity,
            last_tick_ms,
        }
    }

    pub fn into_callback(&self) -> InterComCallback {
        let buffered_sender = self.clone();
        Some(Arc::new(move |response| {
            let buffered_handler = buffered_sender.clone();
            Box::pin({
                async move {
                    let handler = buffered_handler.clone();
                    handler.send(response).await?;
                    Ok(())
                }
            })
        }))
    }

    fn now_as_millis() -> u64 {
        let start = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        start.as_millis() as u64
    }

    pub async fn send(&self, event: InterComEvent) -> anyhow::Result<()> {
        self.buffer
            .entry(event.event_type.clone())
            .or_insert_with(|| Vec::with_capacity(10))
            .push(event);

        let last = self.last_tick_ms.load(Ordering::Relaxed);
        let now = Self::now_as_millis();

        // Flush if:
        // 1. Buffer is at capacity, OR
        // 2. This is the first event, OR
        // 3. Enough time has passed since last flush
        if self.buffer.len() >= self.capacity as usize
            || last == 0
            || (now - last >= self.interval_ms)
        {
            return self.flush().await;
        }

        Ok(())
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let callback = self.callback.clone();

        for mut entry in self.buffer.iter_mut() {
            let events_to_process = std::mem::take(entry.value_mut());

            if !events_to_process.is_empty() {
                if let Err(err) = callback(events_to_process).await {
                    println!("Error publishing events: {}", err);
                }
            }
        }

        self.buffer.retain(|_, events| !events.is_empty());
        self.last_tick_ms
            .store(Self::now_as_millis(), Ordering::Relaxed);
        Ok(())
    }
}

impl Drop for BufferedInterComHandler {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl InterComEvent {
    pub fn new<T>(payload: T) -> Self
    where
        T: Serialize + DeserializeOwned,
    {
        Self {
            event_id: create_id(),
            event_type: "generic".to_string(),
            payload: to_value(payload).unwrap_or(Value::Null),
            timestamp: SystemTime::now(),
        }
    }

    pub fn with_type<T>(event_type: impl Into<String>, payload: T) -> Self
    where
        T: Serialize + DeserializeOwned,
    {
        Self {
            event_id: create_id(),
            event_type: event_type.into(),
            payload: to_value(payload).unwrap_or(Value::Null),
            timestamp: SystemTime::now(),
        }
    }

    pub async fn call(&self, callback: &InterComCallback) -> anyhow::Result<()> {
        if let Some(callback) = callback {
            callback(self.clone()).await?;
        }
        Ok(())
    }
}
