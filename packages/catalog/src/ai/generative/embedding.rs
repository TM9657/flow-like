use std::{any::Any, collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{bit::{Bit, BitTypes}, flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_model_provider::{embedding::EmbeddingModelLogic, image_embedding::ImageEmbeddingModelLogic};
use flow_like_types::{async_trait, json::{from_str, json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Cacheable, Error, JsonSchema, Result, Value};
use load::LoadModelNode;
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

pub mod image;
pub mod load;
pub mod text;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CachedEmbeddingModel {
    pub cache_key: String,
    pub model_type: BitTypes,
}

pub struct CachedEmbeddingModelObject {
    pub text_model: Option<Arc<dyn EmbeddingModelLogic>>,
    pub image_model: Option<Arc<dyn ImageEmbeddingModelLogic>>,
}

impl Cacheable for CachedEmbeddingModelObject {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut nodes: Vec<Arc<dyn NodeLogic>> = vec![Arc::new(LoadModelNode::default())];
    nodes.extend(text::register_functions().await);
    nodes.extend(image::register_functions().await);
    nodes
}
