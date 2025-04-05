pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/flow_like_types.rs"));
}

pub trait ToProto<T> {
    fn to_proto(&self) -> T;
}

pub trait FromProto<T> {
    fn from_proto(proto: T) -> Self;
}

pub trait Cacheable: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl dyn Cacheable {
    pub fn downcast_ref<T: Cacheable>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Cacheable>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

pub type Timestamp = prost_types::Timestamp;

use std::any::Any;

pub use anyhow::{Result, anyhow, bail, Error, Ok};
pub use async_trait::async_trait;
pub use cuid2::create_id;
pub use prost::Message;
pub use reqwest;
pub use schemars::JsonSchema;
pub use serde;
pub use serde_json::Value;
pub mod json {
    pub use serde::{Deserialize, Serialize, de::DeserializeOwned};
    pub use serde_json::{
        from_reader, from_slice, from_str, from_value, json, to_string, to_string_pretty,
        to_value, to_vec, to_vec_pretty, Number, Map
    };
}

pub use tokio;
pub use bytes::Bytes;
pub mod sync {
    pub use tokio::sync::Mutex;
    pub use tokio::sync::RwLock;
    pub use tokio::sync::mpsc;
    pub use dashmap::DashMap;
}

pub use rand;