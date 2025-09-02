/// # ONNX Nodes
/// Loading and Inference for ONNX-based Models
use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_model_provider::ml::ort::session::Session;
use flow_like_types::{Cacheable, Result, create_id, sync::Mutex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// ONNX Image Classification Nodes
pub mod classification;
/// ONNX Image Object Detection Nodes
pub mod detection;
/// ONNX Image Feature Extractor Nodes
pub mod feature;
/// ONNX Model Loader Nodes
pub mod load;

pub enum Provider {
    DfineLike(detection::DfineLike),
    YoloLike(detection::YoloLike),
    TimmLike(classification::TimmLike),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
/// ONNX Runtime Session Reference
pub struct NodeOnnxSession {
    /// Cache ID for Session
    pub session_ref: String,
}

/// ONNX Runtime Session Bundled with Provider Metadata
pub struct SessionWithMeta {
    pub session: Session,
    pub provider: Provider,
}

/// ONNX Runtime Session Wrapper
pub struct NodeOnnxSessionWrapper {
    /// Shared Mutable ONNX Runtime Session
    /// Todo: we might not need a Mutex?
    pub session: Arc<Mutex<SessionWithMeta>>,
}

impl Cacheable for NodeOnnxSessionWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeOnnxSession {
    /// Push new ONNX Runtime Session to Execution Context
    pub async fn new(ctx: &mut ExecutionContext, session: SessionWithMeta) -> Self {
        let id = create_id();
        let session_ref = Arc::new(Mutex::new(session));
        let wrapper = NodeOnnxSessionWrapper {
            session: session_ref.clone(),
        };
        ctx.cache
            .write()
            .await
            .insert(id.clone(), Arc::new(wrapper));
        NodeOnnxSession { session_ref: id }
    }

    /// Fetch ONNX Runtime Session from Cached Runtime Context
    pub async fn get_session(
        &self,
        ctx: &mut ExecutionContext,
    ) -> Result<Arc<Mutex<SessionWithMeta>>> {
        let session = ctx
            .cache
            .read()
            .await
            .get(&self.session_ref)
            .cloned()
            .ok_or_else(|| flow_like_types::anyhow!("ONNX session not found in cache!"))?;
        let session_wrapper = session
            .as_any()
            .downcast_ref::<NodeOnnxSessionWrapper>()
            .ok_or_else(|| {
                flow_like_types::anyhow!("Could not downcast to NodeOnnxSessionWrapper")
            })?;
        let session = session_wrapper.session.clone();
        Ok(session)
    }
}

/// Add ONNX-related Nodes to Catalog Lib
pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(load::LoadOnnxNode::default()),
        Arc::new(detection::ObjectDetectionNode::default()),
        Arc::new(classification::ImageClassificationNode::default()),
    ];
    nodes
}
