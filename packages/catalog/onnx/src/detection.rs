/// # ONNX Object Detection Nodes
use crate::onnx::{NodeOnnxSession, Provider};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_catalog_core::{BoundingBox, NodeImage};
#[cfg(feature = "local-ml")]
use flow_like_model_provider::ml::{
    ndarray::{Array2, Array3, Array4, ArrayView1, Axis, s},
    ort::{
        inputs,
        session::{Session, SessionInputValue, SessionOutputs},
        value::Value,
    },
};
use flow_like_types::{
    Error, Result, anyhow, async_trait,
    image::{DynamicImage, GenericImageView, imageops::FilterType},
    json::json,
};
use std::borrow::Cow;
use std::cmp::Ordering;

#[cfg(feature = "local-ml")]
// ## Object Detection Trait for Common Behavior
pub trait ObjectDetection {
    // Preprocessing
    fn make_inputs(
        &self,
        img: &DynamicImage,
    ) -> Result<Vec<(Cow<'_, str>, SessionInputValue<'_>)>, Error>;
    // Postprocessing
    fn make_results(
        &self,
        outputs: SessionOutputs<'_>,
        conf_thres: f32,
        iou_thres: f32,
        max_detect: usize,
    ) -> Result<Vec<BoundingBox>, Error>;
    // End-to-End Inference
    fn run(
        &self,
        session: &mut Session,
        img: &DynamicImage,
        conf_thres: f32,
        iou_thres: f32,
        max_detect: usize,
    ) -> Result<Vec<BoundingBox>, Error>;
}

// ## Implementation for D-FINE Models
pub struct DfineLike {
    pub input_width: u32,
    pub input_height: u32,
}

#[cfg(feature = "local-ml")]
impl ObjectDetection for DfineLike {
    fn make_inputs(
        &self,
        img: &DynamicImage,
    ) -> Result<Vec<(Cow<'_, str>, SessionInputValue<'_>)>, Error> {
        let (img_width, img_height) = (img.width() as i64, img.height() as i64);
        let images = img_to_arr(img, self.input_width, self.input_height)?;
        let orig_target_size = Array2::from_shape_vec((1, 2), vec![img_width, img_height])?;
        let images_data = Value::from_array(images)?;
        let orig_target_size_data = Value::from_array(orig_target_size)?;
        let session_inputs = inputs![
            "images" => images_data,
            "orig_target_sizes" => orig_target_size_data
        ];
        Ok(session_inputs)
    }

    fn make_results(
        &self,
        outputs: SessionOutputs<'_>,
        conf_thres: f32,
        _iou_thres: f32,
        max_detect: usize,
    ) -> Result<Vec<BoundingBox>, Error> {
        let labels = outputs["labels"].try_extract_array::<i64>()?;
        let boxes = outputs["boxes"].try_extract_array::<f32>()?;
        let scores = outputs["scores"].try_extract_array::<f32>()?;
        let mut bboxes: Vec<BoundingBox> = boxes
            .axis_iter(Axis(1))
            .enumerate()
            .map(|(i, bbox)| {
                let bbox_xyxy = bbox.slice(s![0, ..]).to_vec();
                let (x1, y1, x2, y2) = (bbox_xyxy[0], bbox_xyxy[1], bbox_xyxy[2], bbox_xyxy[3]);
                let class_idx = labels.slice(s![.., i]).to_vec()[0];
                let score = scores.slice(s![.., i]).to_vec()[0];
                BoundingBox {
                    class_idx: class_idx as i32,
                    score,
                    x1,
                    y1,
                    x2,
                    y2,
                    class_name: None,
                }
            })
            .filter(|b| b.score > conf_thres)
            .collect();
        bboxes.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        bboxes.truncate(max_detect);
        Ok(bboxes)
    }

    fn run(
        &self,
        session: &mut Session,
        img: &DynamicImage,
        conf_thres: f32,
        iou_thres: f32,
        max_detect: usize,
    ) -> Result<Vec<BoundingBox>, Error> {
        let session_inputs = self.make_inputs(img)?;
        let session_outputs = session.run(session_inputs)?;
        let bboxes = self.make_results(session_outputs, conf_thres, iou_thres, max_detect)?;
        Ok(bboxes)
    }
}

// ## Implementation for YOLO Models
pub struct YoloLike {
    pub input_width: u32,
    pub input_height: u32,
}

#[cfg(feature = "local-ml")]
impl ObjectDetection for YoloLike {
    fn make_inputs(
        &self,
        img: &DynamicImage,
    ) -> Result<Vec<(Cow<'_, str>, SessionInputValue<'_>)>, Error> {
        let images = img_to_arr(img, self.input_width, self.input_height)?;
        let images_data = Value::from_array(images)?;
        let session_inputs = inputs! [
            "images" => images_data,
        ];
        Ok(session_inputs)
    }

    fn make_results(
        &self,
        outputs: SessionOutputs<'_>,
        conf_thres: f32,
        iou_thres: f32,
        max_detect: usize,
    ) -> Result<Vec<BoundingBox>, Error> {
        let output = outputs["output0"].try_extract_array::<f32>()?;
        let view_candidates = output.slice(s![0, 4.., ..]);
        let mask_candidates: Vec<bool> = view_candidates
            .axis_iter(Axis(1))
            .map(|col| col.iter().cloned().fold(f32::NEG_INFINITY, f32::max) > conf_thres)
            .collect();
        let idx_candidates: Vec<usize> = mask_candidates
            .iter()
            .enumerate()
            .filter_map(|(i, &keep)| if keep { Some(i) } else { None })
            .collect();
        let candidates_image = output.select(Axis(2), &idx_candidates).squeeze();
        let mut bboxes: Vec<BoundingBox> = Vec::with_capacity(candidates_image.len_of(Axis(1)));
        for candidate in candidates_image.axis_iter(Axis(1)) {
            let bbox = bounding_box_from_array(candidate.to_shape(candidate.len()).unwrap().view());
            bboxes.push(bbox);
        }
        let mut bboxes = nms(&bboxes, iou_thres);
        bboxes.truncate(max_detect); // keep only max detections
        Ok(bboxes)
    }

    fn run(
        &self,
        session: &mut Session,
        img: &DynamicImage,
        conf_thres: f32,
        iou_thres: f32,
        max_detect: usize,
    ) -> Result<Vec<BoundingBox>, Error> {
        let session_inputs = self.make_inputs(img)?;
        let session_outputs = session.run(session_inputs)?;
        let mut bboxes = self.make_results(session_outputs, conf_thres, iou_thres, max_detect)?;
        let (target_w, target_h) = (img.width() as f32, img.height() as f32);
        let scale_w = target_w / self.input_width as f32;
        let scale_h = target_h / self.input_height as f32;
        for bbox in &mut bboxes {
            bbox.scale(scale_w, scale_h);
        }
        Ok(bboxes)
    }
}

// ## Detection-Related Utilities

#[cfg(feature = "local-ml")]
/// Load DynamicImage as Array4
/// Resulting normalized 4-dim array has shape [B, C, W, H] (batch size, channels, width, height)
/// ONNX detection model requires Array4-shaped, 0..1 normalized input
fn img_to_arr(img: &DynamicImage, width: u32, height: u32) -> Result<Array4<f32>, Error> {
    let (img_width, img_height) = img.dimensions();

    let buf_u8 = if (img_width == width) && (img_height == height) {
        img.to_rgb8().into_raw()
    } else {
        img.resize_exact(width, height, FilterType::Triangle)
            .into_rgb8()
            .into_raw()
    };

    // to float tensor
    let buf_f32: Vec<f32> = buf_u8.into_iter().map(|v| (v as f32) / 255.0).collect();

    // expand into 4dim array
    let arr4 = Array3::from_shape_vec((height as usize, width as usize, 3), buf_f32)?
        .permuted_axes([2, 0, 1])
        .insert_axis(Axis(0));
    Ok(arr4)
}

/// Convert center-x, center-y, width, height to left, top, right, bottom representation
fn xywh_to_xyxy(x: &f32, y: &f32, w: &f32, h: &f32) -> (f32, f32, f32, f32) {
    let x1 = x - w / 2.0;
    let y1 = y - h / 2.0;
    let x2 = x + w / 2.0;
    let y2 = y + h / 2.0;
    (x1, y1, x2, y2)
}

#[cfg(feature = "local-ml")]
fn bounding_box_from_array(arr: ArrayView1<f32>) -> BoundingBox {
    let bbox_xywh = arr.slice(s![..4]).to_vec();
    let confs = arr.slice(s![4..]).to_vec();
    let (class_idx, conf) = confs
        .iter()
        .enumerate()
        .filter_map(
            |(idx, &num)| {
                if num.is_nan() { None } else { Some((idx, num)) }
            },
        )
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
        .unwrap();
    let (x1, y1, x2, y2) = xywh_to_xyxy(&bbox_xywh[0], &bbox_xywh[1], &bbox_xywh[2], &bbox_xywh[3]);
    BoundingBox {
        x1,
        y1,
        x2,
        y2,
        score: conf,
        class_idx: class_idx as i32,
        class_name: None,
    }
}

/// Class-Sensitive Non Maxima Suppression for Overlapping Bounding Boxes
/// Iteratively removes lower scoring bboxes which have an IoU above iou_thresold.
/// Inspired by: https://pytorch.org/vision/master/_modules/torchvision/ops/boxes.html#nms
fn nms(boxes: &[BoundingBox], iou_threshold: f32) -> Vec<BoundingBox> {
    if boxes.is_empty() {
        return Vec::new();
    }

    // Compute the maximum coordinate value among all boxes
    let max_coordinate = boxes.iter().fold(0.0_f32, |max_coord, bbox| {
        max_coord.max(bbox.x2).max(bbox.y2)
    });
    let offset = max_coordinate + 1.0;

    // Create a vector of shifted boxes with their original indices
    let mut boxes_shifted: Vec<(BoundingBox, usize)> = boxes
        .iter()
        .enumerate()
        .map(|(i, bbox)| {
            let class_offset = offset * bbox.class_idx as f32;
            let shifted_bbox = BoundingBox {
                x1: bbox.x1 + class_offset,
                y1: bbox.y1 + class_offset,
                x2: bbox.x2 + class_offset,
                y2: bbox.y2 + class_offset,
                score: bbox.score,
                class_idx: bbox.class_idx, // Keep class_idx the same
                class_name: None,
            };
            (shifted_bbox, i) // Keep track of the original index
        })
        .collect();

    // Sort boxes in decreasing order based on scores
    boxes_shifted
        .sort_unstable_by(|a, b| b.0.score.partial_cmp(&a.0.score).unwrap_or(Ordering::Equal));

    let mut keep_indices = Vec::new();

    while let Some((current_box, original_index)) = boxes_shifted.first().cloned() {
        keep_indices.push(original_index);
        boxes_shifted.remove(0);

        // Retain boxes that have an IoU less than or equal to the threshold with the current box
        boxes_shifted.retain(|(bbox, _)| current_box.iou(bbox) <= iou_threshold);
    }

    // Collect the kept boxes from the original input
    let mut kept_boxes: Vec<BoundingBox> = keep_indices
        .into_iter()
        .map(|idx| boxes[idx].clone())
        .collect();

    // Sort the kept boxes in decreasing order of their scores
    kept_boxes.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    kept_boxes
}

#[crate::register_node]
#[derive(Default)]
/// # Object Detection Node
/// Evaluate ONNX-based Object Detection Models for Images
pub struct ObjectDetectionNode {}

impl ObjectDetectionNode {
    /// Create new LoadOnnxNode Instance
    pub fn new() -> Self {
        ObjectDetectionNode {}
    }
}

#[async_trait]
impl NodeLogic for ObjectDetectionNode {
    fn get_node(&self) -> Node {
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

        node.add_input_pin("model", "Model", "ONNX Model Session", VariableType::Struct)
            .set_schema::<NodeOnnxSession>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("image_in", "Image", "Image Object", VariableType::Struct)
            .set_schema::<NodeImage>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("conf", "Conf", "Confidence Threshold", VariableType::Float)
            .set_options(PinOptions::new().set_range((0., 1.)).build())
            .set_default_value(Some(json!(0.25)));

        node.add_input_pin(
            "iou",
            "IoU",
            "Intersection Over Union Threshold for NMS",
            VariableType::Float,
        )
        .set_options(PinOptions::new().set_range((0., 1.)).build())
        .set_default_value(Some(json!(0.7)));

        node.add_input_pin(
            "max",
            "Max",
            "Maximum Number of Detections",
            VariableType::Integer,
        )
        .set_options(PinOptions::new().set_range((0., 1000.)).build())
        .set_default_value(Some(json!(300)));

        // outputs
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "bboxes",
            "Boxes",
            "Bounding Box Predictions",
            VariableType::Struct,
        )
        .set_schema::<BoundingBox>()
        .set_value_type(flow_like::flow::pin::ValueType::Array);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "local-ml")]
        {
            context.deactivate_exec_pin("exec_out").await?;

            // fetch params
            let node_session: NodeOnnxSession = context.evaluate_pin("model").await?;
            let node_img: NodeImage = context.evaluate_pin("image_in").await?;
            let conf_thres: f32 = context.evaluate_pin("conf").await?;
            let iou_thres: f32 = context.evaluate_pin("iou").await?;
            let max_detect: usize = context.evaluate_pin("max").await?;

            // run inference
            let predictions = {
                let img = node_img.get_image(context).await?;
                let img_guard = img.lock().await;
                let session = node_session.get_session(context).await?;
                let mut session_guard = session.lock().await;
                // Copy provider params to avoid overlapping borrows
                match &session_guard.provider {
                    Provider::DfineLike(m) => {
                        let prov = super::detection::DfineLike {
                            input_width: m.input_width,
                            input_height: m.input_height,
                        };
                        prov.run(
                            &mut session_guard.session,
                            &img_guard,
                            conf_thres,
                            iou_thres,
                            max_detect,
                        )
                    }
                    Provider::YoloLike(m) => {
                        let prov = super::detection::YoloLike {
                            input_width: m.input_width,
                            input_height: m.input_height,
                        };
                        prov.run(
                            &mut session_guard.session,
                            &img_guard,
                            conf_thres,
                            iou_thres,
                            max_detect,
                        )
                    }
                    _ => Err(anyhow!(
                        "Unknown/Incompatible ONNX-Model for Object Detection!"
                    )),
                }?
            };

            // set outputs
            context.set_pin_value("bboxes", json!(predictions)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "local-ml"))]
        {
            Err(anyhow!(
                "Local ONNX models are not supported. Please enable the 'local-ml' feature."
            ))
        }
    }
}
