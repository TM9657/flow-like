use crate::{image::NodeImage, storage::path::FlowPath};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{
    JsonSchema, Result, async_trait,
    image::imageops::FilterType,
    json::{Deserialize, Serialize, json},
    tokio,
};
use std::io::Cursor;
use tract_tflite::prelude::*;

#[derive(Default, Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ClassPrediction {
    pub class_idx: u32,
    pub score: f32,
    pub label: Option<String>,
}

#[derive(Default)]
pub struct TfliteImageClassificationNode {}

impl TfliteImageClassificationNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for TfliteImageClassificationNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "tf_savedmodel_image_classification",
            "TensorFlow Lite Image Classification",
            "Image classification using TensorFlow Lite models (*.tflite).",
            "AI/ML/TensorFlow",
        );

        node.add_icon("/flow/icons/find_model.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "model",
            "Model File",
            "Path to *.tflite model",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("image_in", "Image", "Image Object", VariableType::Struct)
            .set_schema::<NodeImage>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "labels",
            "Labels",
            "Optional labels.txt",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "input_width",
            "Input Width",
            "Model input width",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(224)));
        node.add_input_pin(
            "input_height",
            "Input Height",
            "Model input height",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(224)));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );
        node.add_output_pin(
            "predictions",
            "Predictions",
            "Class Predictions",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let model_path: FlowPath = context.evaluate_pin("model").await?;
        let node_img: NodeImage = context.evaluate_pin("image_in").await?;
        let labels_path_opt: Option<FlowPath> = context.evaluate_pin("labels").await.ok();
        let iw: i64 = context.evaluate_pin("input_width").await.unwrap_or(224);
        let ih: i64 = context.evaluate_pin("input_height").await.unwrap_or(224);
        let (iw, ih) = (iw.max(1) as u32, ih.max(1) as u32);

        let labels_vec: Vec<String> = if let Some(labels_path) = labels_path_opt {
            let lbs = labels_path.get(context, false).await?;
            let s = String::from_utf8(lbs)
                .map_err(|e| flow_like_types::anyhow!("Failed to parse labels file: {}", e))?;
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        let raw = model_path.get(context, false).await.map_err(|e| {
            flow_like_types::anyhow!("Failed to load .tflite model '{}': {}", model_path.path, e)
        })?;
        if raw.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Model file '{}' is empty",
                model_path.path
            ));
        }

        let model_bytes: Vec<u8> = find_tflite_slice(&raw)
            .ok_or_else(|| flow_like_types::anyhow!("Could not locate TFLite buffer (TFL3 id)"))?
            .to_vec();

        let img = node_img.get_image(context).await?;
        let img_guard = img.lock().await;
        let resized = img_guard
            .resize_exact(iw, ih, FilterType::CatmullRom)
            .to_rgb8();
        drop(img_guard);

        let mut input_f32 = Vec::with_capacity((ih * iw * 3) as usize);
        for y in 0..ih {
            for x in 0..iw {
                let p = resized.get_pixel(x, y);
                input_f32.push(p[0] as f32 / 255.0);
                input_f32.push(p[1] as f32 / 255.0);
                input_f32.push(p[2] as f32 / 255.0);
            }
        }

        let (top_idx, top_score) =
            tokio::task::spawn_blocking(move || -> flow_like_types::Result<(usize, f32)> {
                let tfl = tract_tflite::tflite();
                let mut cursor = Cursor::new(model_bytes);
                let mut model = tract_tflite::tflite()
                    .model_for_read(&mut cursor)
                    .map_err(|e| flow_like_types::anyhow!("TFLite parse error: {e}"))?;

                // Peek original input fact
                let inlet = model.input_outlets()?[0];
                let orig = model.outlet_fact(inlet)?;
                use tract_data::prelude::DatumType;

                let fact = match orig.datum_type {
                    DatumType::F32 => TypedFact::dt_shape(
                        f32::datum_type(),
                        tvec!(1, ih as usize, iw as usize, 3),
                    ),
                    DatumType::U8 => {
                        TypedFact::dt_shape(u8::datum_type(), tvec!(1, ih as usize, iw as usize, 3))
                    }
                    DatumType::I8 => {
                        TypedFact::dt_shape(i8::datum_type(), tvec!(1, ih as usize, iw as usize, 3))
                    }
                    dt => return Err(flow_like_types::anyhow!("Unsupported input dtype: {dt:?}")),
                };
                let mut runnable = model
                    .with_input_fact(0, fact)?
                    .into_optimized()?
                    .into_runnable()?;

                let input: Tensor = tract_ndarray::Array4::from_shape_vec(
                    (1, ih as usize, iw as usize, 3),
                    input_f32,
                )
                .map_err(|e| flow_like_types::anyhow!("Failed to shape input tensor: {e}"))?
                .into();

                let outputs = runnable
                    .run(tvec!(input.into()))
                    .map_err(|e| flow_like_types::anyhow!("Failed to run TFLite model: {e}"))?;

                if outputs.is_empty() {
                    return Err(flow_like_types::anyhow!("Model produced no outputs"));
                }

                let out = outputs[0]
                    .to_array_view::<f32>()
                    .map_err(|e| flow_like_types::anyhow!("Output is not f32: {}", e))?;

                let mut best_idx = 0usize;
                let mut best_score = f32::MIN;
                for (i, v) in out.iter().enumerate() {
                    if *v > best_score {
                        best_idx = i;
                        best_score = *v;
                    }
                }
                Ok((best_idx, best_score))
            })
            .await
            .map_err(|e| flow_like_types::anyhow!("TFLite inference task join error: {}", e))??;

        let label = labels_vec.get(top_idx).cloned();
        let predictions = vec![ClassPrediction {
            class_idx: top_idx as u32,
            score: top_score,
            label,
        }];

        context
            .set_pin_value("predictions", json!(predictions))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

fn find_tflite_slice(buf: &[u8]) -> Option<&[u8]> {
    if buf.len() < 8 {
        return None;
    }
    let limit = buf.len() - 8;
    for i in 0..=limit {
        if &buf[i + 4..i + 8] == b"TFL3" {
            return Some(&buf[i..]);
        }
    }
    None
}
