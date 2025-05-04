/// # ONNX Object Detection Nodes

use crate::{
    ai::ml::onnx::NodeOnnxSession, 
    image::NodeImage,
};
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{
    anyhow, async_trait, image::{imageops::FilterType, DynamicImage}, ndarray::{Array3, Array4, Axis}, ort, Error, Result
};

/// # Load DynamicImage as Array4
/// Resulting normalized 4-dim array has shape [B, C, W, H] (batch size, channels, width, height)
/// ONNX detection model requires Array4-shaped, 0..1 normalized input
fn img_to_arr(img: &DynamicImage) -> Result<Array4<f32>, Error> {
    let (width, height) = (640, 640);
    let buf_u8 = img
        .resize_exact(width, height, FilterType::Triangle)
        .to_rgb8()
        .into_raw();

    let buf_f32: Vec<f32> = buf_u8
        .into_iter()
        .map(|v| (v as f32) / 255.0)
        .collect();

    let arr4 = Array3::from_shape_vec((height as usize, width as usize, 3), buf_f32)?
        .permuted_axes([2, 0, 1])
        .insert_axis(Axis(0));

    Ok(arr4)
}


#[derive(Default)]
/// # Object Detection Node
/// Evaluate ONNX-based Object Detection Models for Images
pub struct ObjectDetectionNode {}

impl ObjectDetectionNode {
    /// Create new LoadOnnxNode Instance
    pub fn new() -> Self {
        ObjectDetectionNode {  }
    }
}

#[async_trait]
impl NodeLogic for ObjectDetectionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "object_detection",
            "Object Detection",
            "Object Detection in Images with ONNX-Models",
            "AI/ML/ONNX",
        );

        node.add_icon("/flow/icons/find_model.svg");

        // inputs
        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "model", 
            "Model", 
            "ONNX Model Session", 
            VariableType::Struct
        )
            .set_schema::<NodeOnnxSession>()
            .set_options(PinOptions::new()
                .set_enforce_schema(true)
                .build()
            );

        node.add_input_pin(
            "image_in", 
            "Image", 
            "Image object", 
            VariableType::Struct
        )
            .set_schema::<NodeImage>()
            .set_options(PinOptions::new()
                .set_enforce_schema(true)
                .build()
            );

        
        // outputs 
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "prediction",
            "Prediction",
            "Object Detection Response",
            VariableType::Struct,
        );

        node

    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        
        // fetch cached 
        let node_img: NodeImage = context.evaluate_pin("image_in").await?;
        let img = node_img.get_image(context).await?;
        let img_guard = img.lock().await;

        let node_session: NodeOnnxSession = context.evaluate_pin("model").await?;
        let session = node_session.get_session(context).await?;
        let session_guard = session.lock().await;
        
        // prepare ONNX-model input
        let arr = img_to_arr(&img_guard)?;
        context.log_message(&format!("input array: {:?}", arr.shape()), LogLevel::Debug);
        let inputs = match ort::inputs!["images" => arr.view()] {
            Ok(mapping) => Ok(mapping),
            Err(_) => Err(anyhow!("failed to prepare onnx model input")),
        }?;

        // run inference
        let outputs = session_guard.run(inputs)?;
        let arr_out = outputs["output0"].try_extract_tensor::<f32>()?;
        context.log_message(&format!("output array: {:?}", arr_out.shape()), LogLevel::Debug);

        // set outputs
        context.activate_exec_pin("exec_out").await?;
        Ok(())

    }
}