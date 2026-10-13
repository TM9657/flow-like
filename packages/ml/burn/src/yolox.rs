use crate::{BoundingBox, Detection, DetectionOptions, Result, TensorData};
use burn::{
    module::{Module, Param},
    nn::{
        BatchNorm, BatchNormConfig, PaddingConfig2d,
        conv::{Conv2d, Conv2dConfig},
        interpolate::Interpolate2dConfig,
        pool::MaxPool2dConfig,
    },
    tensor::{
        Device, Int, Tensor, TensorData as BurnData,
        activation::{log_sigmoid, sigmoid},
    },
};

#[derive(Module, Debug)]
struct BaseConv {
    conv: Conv2d,
    norm: BatchNorm,
}
impl BaseConv {
    fn new(input: usize, output: usize, kernel: usize, stride: usize, device: &Device) -> Self {
        let p = kernel / 2;
        Self {
            conv: Conv2dConfig::new([input, output], [kernel, kernel])
                .with_stride([stride, stride])
                .with_padding(PaddingConfig2d::Explicit(p, p, p, p))
                .with_bias(false)
                .init(device),
            norm: BatchNormConfig::new(output).init(device),
        }
    }
    fn forward(&self, x: Tensor<4>) -> Tensor<4> {
        let x = self.norm.forward(self.conv.forward(x));
        x.clone() * sigmoid(x)
    }
}
#[derive(Module, Debug)]
struct Bottleneck {
    first: BaseConv,
    second: BaseConv,
    shortcut: bool,
}
impl Bottleneck {
    fn new(channels: usize, shortcut: bool, device: &Device) -> Self {
        Self {
            first: BaseConv::new(channels, channels, 1, 1, device),
            second: BaseConv::new(channels, channels, 3, 1, device),
            shortcut,
        }
    }
    fn forward(&self, x: Tensor<4>) -> Tensor<4> {
        let output = self.second.forward(self.first.forward(x.clone()));
        if self.shortcut { output + x } else { output }
    }
}
#[derive(Module, Debug)]
struct Csp {
    left: BaseConv,
    right: BaseConv,
    blocks: Vec<Bottleneck>,
    merge: BaseConv,
}
impl Csp {
    fn new(input: usize, output: usize, depth: usize, shortcut: bool, device: &Device) -> Self {
        let hidden = output / 2;
        Self {
            left: BaseConv::new(input, hidden, 1, 1, device),
            right: BaseConv::new(input, hidden, 1, 1, device),
            blocks: (0..depth)
                .map(|_| Bottleneck::new(hidden, shortcut, device))
                .collect(),
            merge: BaseConv::new(hidden * 2, output, 1, 1, device),
        }
    }
    fn forward(&self, x: Tensor<4>) -> Tensor<4> {
        let right = self.right.forward(x.clone());
        let mut left = self.left.forward(x);
        for block in &self.blocks {
            left = block.forward(left);
        }
        self.merge.forward(Tensor::cat(vec![left, right], 1))
    }
}
#[derive(Module, Debug)]
struct Spp {
    first: BaseConv,
    last: BaseConv,
}
impl Spp {
    fn new(channels: usize, device: &Device) -> Self {
        Self {
            first: BaseConv::new(channels, channels / 2, 1, 1, device),
            last: BaseConv::new(channels * 2, channels, 1, 1, device),
        }
    }
    fn forward(&self, x: Tensor<4>) -> Tensor<4> {
        let x = self.first.forward(x);
        let mut features = vec![x.clone()];
        for size in [5, 9, 13] {
            let p = size / 2;
            features.push(
                MaxPool2dConfig::new([size, size])
                    .with_strides([1, 1])
                    .with_padding(PaddingConfig2d::Explicit(p, p, p, p))
                    .init()
                    .forward(x.clone()),
            );
        }
        self.last.forward(Tensor::cat(features, 1))
    }
}
#[derive(Module, Debug)]
struct Head {
    stem: BaseConv,
    cls1: BaseConv,
    cls2: BaseConv,
    reg1: BaseConv,
    reg2: BaseConv,
    boxes: Conv2d,
    objectness: Conv2d,
    classes: Conv2d,
}
impl Head {
    fn new(input: usize, hidden: usize, classes: usize, device: &Device) -> Self {
        Self {
            stem: BaseConv::new(input, hidden, 1, 1, device),
            cls1: BaseConv::new(hidden, hidden, 3, 1, device),
            cls2: BaseConv::new(hidden, hidden, 3, 1, device),
            reg1: BaseConv::new(hidden, hidden, 3, 1, device),
            reg2: BaseConv::new(hidden, hidden, 3, 1, device),
            boxes: Conv2dConfig::new([hidden, 4], [1, 1]).init(device),
            objectness: prior_conv(hidden, 1, device),
            classes: prior_conv(hidden, classes, device),
        }
    }
    fn forward(&self, x: Tensor<4>) -> Tensor<3> {
        let x = self.stem.forward(x);
        let cls = self
            .classes
            .forward(self.cls2.forward(self.cls1.forward(x.clone())));
        let reg = self.reg2.forward(self.reg1.forward(x));
        Tensor::cat(
            vec![
                self.boxes.forward(reg.clone()),
                self.objectness.forward(reg),
                cls,
            ],
            1,
        )
        .flatten(2, 3)
        .swap_dims(1, 2)
    }
}
/// Score biases start at the upstream 1% foreground prior so the dense negative objectness loss
/// does not dominate early training.
fn prior_conv(input: usize, output: usize, device: &Device) -> Conv2d {
    let mut conv = Conv2dConfig::new([input, output], [1, 1]).init(device);
    conv.bias = Some(Param::from_tensor(Tensor::full(
        [output],
        (0.01f32 / 0.99).ln(),
        device,
    )));
    conv
}

/// CSPDarknet, path aggregation feature pyramid and decoupled YOLOX heads.
#[derive(Module, Debug)]
pub struct YoloX {
    focus: BaseConv,
    down: Vec<BaseConv>,
    backbone: Vec<Csp>,
    spp: Spp,
    lateral: BaseConv,
    top4: Csp,
    reduce: BaseConv,
    top3: Csp,
    bottom3: BaseConv,
    merge4: Csp,
    bottom4: BaseConv,
    merge5: Csp,
    heads: Vec<Head>,
}
impl YoloX {
    pub fn new(input: usize, classes: usize, width: f64, depth: f64, device: &Device) -> Self {
        let base = ((64.0 * width) as usize / 2 * 2).max(4);
        let n = (3.0 * depth).round().max(1.0) as usize;
        let c3 = base * 4;
        let c4 = base * 8;
        let c5 = base * 16;
        let down = (0..4)
            .map(|level| BaseConv::new(base << level, base << (level + 1), 3, 2, device))
            .collect();
        let backbone = (0..4)
            .map(|level| {
                Csp::new(
                    base << (level + 1),
                    base << (level + 1),
                    if level == 1 || level == 2 { n * 3 } else { n },
                    level != 3,
                    device,
                )
            })
            .collect();
        Self {
            focus: BaseConv::new(input * 4, base, 3, 1, device),
            down,
            backbone,
            spp: Spp::new(c5, device),
            lateral: BaseConv::new(c5, c4, 1, 1, device),
            top4: Csp::new(c4 * 2, c4, n, false, device),
            reduce: BaseConv::new(c4, c3, 1, 1, device),
            top3: Csp::new(c3 * 2, c3, n, false, device),
            bottom3: BaseConv::new(c3, c3, 3, 2, device),
            merge4: Csp::new(c3 * 2, c4, n, false, device),
            bottom4: BaseConv::new(c4, c4, 3, 2, device),
            merge5: Csp::new(c4 * 2, c5, n, false, device),
            heads: [c3, c4, c5]
                .into_iter()
                .map(|input| Head::new(input, c3, classes, device))
                .collect(),
        }
    }
    pub fn forward(&self, input: Tensor<4>) -> YoloOutput {
        let device = input.device();
        let [b, c, h, w] = input.dims();
        let mut x = self.focus.forward(
            input
                .reshape([b, c, h / 2, 2, w / 2, 2])
                .permute([0, 5, 3, 1, 2, 4])
                .reshape([b, c * 4, h / 2, w / 2]),
        );
        let mut features = Vec::new();
        for i in 0..4 {
            x = self.down[i].forward(x);
            if i == 3 {
                x = self.spp.forward(x);
            }
            x = self.backbone[i].forward(x);
            if i > 0 {
                features.push(x.clone());
            }
        }
        let c3 = features.remove(0);
        let c4 = features.remove(0);
        let c5 = features.remove(0);
        let p5 = self.lateral.forward(c5);
        let p4 = self
            .top4
            .forward(Tensor::cat(vec![upsample(p5.clone(), &c4), c4], 1));
        let p4_reduced = self.reduce.forward(p4);
        let p3 = self
            .top3
            .forward(Tensor::cat(vec![upsample(p4_reduced.clone(), &c3), c3], 1));
        let n4 = self.merge4.forward(Tensor::cat(
            vec![self.bottom3.forward(p3.clone()), p4_reduced],
            1,
        ));
        let n5 = self
            .merge5
            .forward(Tensor::cat(vec![self.bottom4.forward(n4.clone()), p5], 1));
        let mut grids = Vec::new();
        let mut scales = Vec::new();
        let mut raw = Vec::new();
        for (i, feature) in [p3, n4, n5].into_iter().enumerate() {
            let [_, _, fh, fw] = feature.dims();
            let stride = (8usize << i) as f32;
            for gy in 0..fh {
                for gx in 0..fw {
                    grids.extend([gx as f32, gy as f32]);
                    scales.extend([stride / w as f32, stride / h as f32]);
                }
            }
            raw.push(self.heads[i].forward(feature));
        }
        let logits = Tensor::cat(raw, 1);
        let count = logits.dims()[1];
        YoloOutput {
            logits,
            grid: Tensor::from_data(BurnData::new(grids, [1, count, 2]), &device),
            scale: Tensor::from_data(BurnData::new(scales, [1, count, 2]), &device),
        }
    }
}
fn upsample(x: Tensor<4>, target: &Tensor<4>) -> Tensor<4> {
    let [_, _, h, w] = target.dims();
    Interpolate2dConfig::new()
        .with_output_size(Some([h, w]))
        .init()
        .forward(x)
}

pub(crate) struct YoloOutput {
    logits: Tensor<3>,
    grid: Tensor<3>,
    scale: Tensor<3>,
}
impl YoloOutput {
    #[cfg(feature = "onnx-export")]
    pub(crate) fn raw_logits(self) -> Tensor<3> {
        self.logits
    }

    fn decoded(&self) -> Tensor<3> {
        let [b, n, _] = self.logits.dims();
        let xy = (self.logits.clone().slice([0..b, 0..n, 0..2]) + self.grid.clone())
            * self.scale.clone();
        let wh = self
            .logits
            .clone()
            .slice([0..b, 0..n, 2..4])
            .clamp(-10.0, 10.0)
            .exp()
            * self.scale.clone();
        Tensor::cat(vec![xy - wh.clone() * 0.5, wh], 2)
    }
    fn simota_inputs(&self, decoded: &Tensor<3>) -> Result<[Vec<f32>; 4]> {
        Ok([
            read(decoded.clone())?,
            read(self.logits.clone())?,
            read(self.grid.clone())?,
            read(self.scale.clone())?,
        ])
    }
    pub(crate) fn loss(self, targets: &[Vec<BoundingBox>]) -> Result<Tensor<1>> {
        let device = self.logits.device();
        let [batch, count, columns] = self.logits.dims();
        let classes = columns - 5;
        let decoded = self.decoded();
        let [boxes, raw, grid, scale] = self.simota_inputs(&decoded)?;
        let mut object_targets = vec![0.0f32; batch * count];
        let mut selected = Vec::new();
        let mut target_boxes = Vec::new();
        let mut class_targets = Vec::new();
        for (b, ground_truth) in targets.iter().enumerate() {
            for (anchor, target_index, quality) in simota(
                &boxes[b * count * 4..(b + 1) * count * 4],
                &raw[b * count * columns..(b + 1) * count * columns],
                &grid,
                &scale,
                ground_truth,
                classes,
            ) {
                let target = &ground_truth[target_index];
                object_targets[b * count + anchor] = 1.0;
                selected.push((b * count + anchor) as i64);
                target_boxes.extend([
                    target.x_min,
                    target.y_min,
                    target.x_max - target.x_min,
                    target.y_max - target.y_min,
                ]);
                for c in 0..classes {
                    class_targets.push(if c == target.class_id { quality } else { 0.0 });
                }
            }
        }
        let positives = selected.len();
        let denom = positives.max(1) as f32;
        let obj = self
            .logits
            .clone()
            .slice([0..batch, 0..count, 4..5])
            .reshape([batch, count]);
        let object_loss = bce(
            obj,
            Tensor::from_data(BurnData::new(object_targets, [batch, count]), &device),
        )
        .sum()
            / denom;
        if positives == 0 {
            return Ok(object_loss);
        }
        let indices = Tensor::<1, Int>::from_data(BurnData::new(selected, [positives]), &device);
        let pred_boxes = decoded
            .reshape([batch * count, 4])
            .select(0, indices.clone());
        let target_boxes =
            Tensor::<2>::from_data(BurnData::new(target_boxes, [positives, 4]), &device);
        let quality = iou_tensor(pred_boxes, target_boxes);
        let box_loss = (quality.square() * -1.0 + 1.0).sum() / denom;
        let cls = self
            .logits
            .reshape([batch * count, columns])
            .select(0, indices)
            .slice([0..positives, 5..columns]);
        let cls_loss = bce(
            cls,
            Tensor::from_data(BurnData::new(class_targets, [positives, classes]), &device),
        )
        .sum()
            / denom;
        Ok(object_loss + box_loss * 5.0 + cls_loss)
    }
    pub(crate) fn prediction(
        self,
        options: &DetectionOptions,
    ) -> Result<(TensorData, Vec<Vec<Detection>>)> {
        let shape = self.logits.dims();
        let [batch, count, columns] = shape;
        let classes = columns - 5;
        let boxes = read(self.decoded())?;
        let values = read(self.logits)?;
        let mut result = Vec::new();
        for b in 0..batch {
            let mut detections = Vec::new();
            for i in 0..count {
                let offset = (b * count + i) * columns;
                let class = (0..classes)
                    .max_by(|a, c| values[offset + 5 + a].total_cmp(&values[offset + 5 + c]))
                    .unwrap_or(0);
                let confidence =
                    logistic(values[offset + 4]) * logistic(values[offset + 5 + class]);
                if confidence < options.confidence_threshold {
                    continue;
                }
                let offset = (b * count + i) * 4;
                let x = boxes[offset];
                let y = boxes[offset + 1];
                let width = boxes[offset + 2];
                let height = boxes[offset + 3];
                detections.push(Detection {
                    class_id: class,
                    confidence,
                    x_min: x.clamp(0.0, 1.0),
                    y_min: y.clamp(0.0, 1.0),
                    x_max: (x + width).clamp(0.0, 1.0),
                    y_max: (y + height).clamp(0.0, 1.0),
                });
            }
            result.push(nms(detections, options));
        }
        let output = TensorData {
            shape: shape.to_vec(),
            values,
        };
        output.validate()?;
        Ok((output, result))
    }
}
fn read<const D: usize>(tensor: Tensor<D>) -> Result<Vec<f32>> {
    Ok(tensor
        .try_into_data()
        .map_err(crate::execution::device_error)?
        .iter::<f32>()
        .collect())
}
fn bce<const D: usize>(logits: Tensor<D>, targets: Tensor<D>) -> Tensor<D> {
    -log_sigmoid(logits.clone()) * targets.clone() - log_sigmoid(-logits) * (-targets + 1.0)
}
fn iou_tensor(a: Tensor<2>, b: Tensor<2>) -> Tensor<1> {
    let n = a.dims()[0];
    let axy = a.clone().slice([0..n, 0..2]);
    let awh = a.slice([0..n, 2..4]);
    let bxy = b.clone().slice([0..n, 0..2]);
    let bwh = b.slice([0..n, 2..4]);
    let size = (axy.clone() + awh.clone()).min_pair(bxy.clone() + bwh.clone()) - axy.max_pair(bxy);
    let size = size.clamp_min(0.0);
    let intersection = size.clone().slice([0..n, 0..1]) * size.slice([0..n, 1..2]);
    let area_a = awh.clone().slice([0..n, 0..1]) * awh.slice([0..n, 1..2]);
    let area_b = bwh.clone().slice([0..n, 0..1]) * bwh.slice([0..n, 1..2]);
    (intersection.clone() / (area_a + area_b - intersection + 1e-8)).squeeze_dim(1)
}
fn logistic(x: f32) -> f32 {
    1.0 / (1.0 + (-x.clamp(-60.0, 60.0)).exp())
}
fn box_iou(a: [f32; 4], b: [f32; 4]) -> f32 {
    let intersection =
        (a[2].min(b[2]) - a[0].max(b[0])).max(0.0) * (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let area_a = (a[2] - a[0]).max(0.0) * (a[3] - a[1]).max(0.0);
    let area_b = (b[2] - b[0]).max(0.0) * (b[3] - b[1]).max(0.0);
    intersection / (area_a + area_b - intersection + 1e-8)
}
/// Dynamic positive assignment runs outside autodiff; losses retain gradients through selected predictions.
fn simota(
    boxes: &[f32],
    raw: &[f32],
    grid: &[f32],
    scale: &[f32],
    targets: &[BoundingBox],
    classes: usize,
) -> Vec<(usize, usize, f32)> {
    let count = boxes.len() / 4;
    let mut assignment = vec![None::<(usize, f32, f32)>; count];
    for (target_index, target) in targets.iter().enumerate() {
        let gt = [target.x_min, target.y_min, target.x_max, target.y_max];
        let center = [(gt[0] + gt[2]) * 0.5, (gt[1] + gt[3]) * 0.5];
        let mut costs = Vec::new();
        for anchor in 0..count {
            let center_x = (grid[anchor * 2] + 0.5) * scale[anchor * 2];
            let center_y = (grid[anchor * 2 + 1] + 0.5) * scale[anchor * 2 + 1];
            let in_box =
                center_x > gt[0] && center_x < gt[2] && center_y > gt[1] && center_y < gt[3];
            let in_center = (center_x - center[0]).abs() < 2.5 * scale[anchor * 2]
                && (center_y - center[1]).abs() < 2.5 * scale[anchor * 2 + 1];
            if !in_box && !in_center {
                continue;
            }
            let b = &boxes[anchor * 4..anchor * 4 + 4];
            let quality = box_iou([b[0], b[1], b[0] + b[2], b[1] + b[3]], gt);
            let offset = anchor * (classes + 5);
            let obj = logistic(raw[offset + 4]);
            let mut cls_cost = 0.0;
            for c in 0..classes {
                let p = (obj * logistic(raw[offset + 5 + c]))
                    .sqrt()
                    .clamp(1e-7, 1.0 - 1e-7);
                cls_cost -= if c == target.class_id {
                    p.ln()
                } else {
                    (1.0 - p).ln()
                };
            }
            let cost = cls_cost - 3.0 * (quality + 1e-8).ln()
                + if in_box && in_center { 0.0 } else { 100_000.0 };
            costs.push((anchor, cost, quality));
        }
        let mut qualities = costs.iter().map(|(_, _, q)| *q).collect::<Vec<_>>();
        qualities.sort_by(|a, b| b.total_cmp(a));
        let k = (qualities.iter().take(10).sum::<f32>() as usize)
            .max(1)
            .min(costs.len());
        costs.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (anchor, cost, quality) in costs.into_iter().take(k) {
            if assignment[anchor]
                .as_ref()
                .is_none_or(|(_, old, _)| cost < *old)
            {
                assignment[anchor] = Some((target_index, cost, quality));
            }
        }
    }
    assignment
        .into_iter()
        .enumerate()
        .filter_map(|(anchor, m)| m.map(|(target, _, quality)| (anchor, target, quality)))
        .collect()
}
fn nms(mut detections: Vec<Detection>, options: &DetectionOptions) -> Vec<Detection> {
    detections.retain(|d| d.x_max > d.x_min && d.y_max > d.y_min);
    detections.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    let mut kept = Vec::<Detection>::new();
    for detection in detections {
        if kept.iter().any(|old| {
            old.class_id == detection.class_id
                && box_iou(
                    [old.x_min, old.y_min, old.x_max, old.y_max],
                    [
                        detection.x_min,
                        detection.y_min,
                        detection.x_max,
                        detection.y_max,
                    ],
                ) > options.iou_threshold
        }) {
            continue;
        }
        kept.push(detection);
        if kept.len() >= options.max_detections {
            break;
        }
    }
    kept
}

#[cfg(all(test, feature = "cpu"))]
mod tests {
    use super::*;
    #[test]
    fn bce_handles_zero_and_extreme_logits_with_correct_gradients() {
        let _lock = crate::tests::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let device = Device::flex().autodiff();
        let logits = Tensor::<1>::from_data([0.0f32, 0.0, -100.0, 100.0, -100.0, 100.0], &device)
            .require_grad();
        let targets = Tensor::<1>::from_data([0.0f32, 1.0, 0.0, 1.0, 1.0, 0.0], &device);
        let loss = bce(logits.clone(), targets);
        let values = loss.clone().into_data().try_to_vec::<f32>().unwrap();
        let expected = [
            std::f32::consts::LN_2,
            std::f32::consts::LN_2,
            0.0,
            0.0,
            100.0,
            100.0,
        ];
        for (actual, expected) in values.into_iter().zip(expected) {
            assert!(actual.is_finite() && (actual - expected).abs() < 1e-6);
        }
        let gradients = logits
            .grad(&loss.sum().backward())
            .unwrap()
            .into_data()
            .try_to_vec::<f32>()
            .unwrap();
        for (actual, expected) in gradients.into_iter().zip([0.5, -0.5, 0.0, 0.0, -1.0, 1.0]) {
            assert!(
                actual.is_finite() && (actual - expected).abs() < 1e-6,
                "gradient {actual} != {expected}"
            );
        }
    }
    #[test]
    fn new_score_heads_start_at_the_upstream_foreground_prior() {
        let _lock = crate::tests::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let device = Device::flex();
        let model = YoloX::new(1, 3, 0.0625, 0.33, &device);
        let prior = (0.01f32 / 0.99).ln();
        for head in &model.heads {
            for (layer, channels) in [(&head.objectness, 1), (&head.classes, 3)] {
                let bias = layer
                    .bias
                    .as_ref()
                    .unwrap()
                    .val()
                    .into_data()
                    .iter::<f32>()
                    .collect::<Vec<_>>();
                assert_eq!(bias.len(), channels);
                assert!(bias.iter().all(|value| (value - prior).abs() < 1e-6));
            }
        }
        // A blank image has zero head features, so every score logit is exactly the prior.
        let logits = model
            .forward(Tensor::zeros([1, 1, 32, 32], &device))
            .logits
            .into_data()
            .iter::<f32>()
            .collect::<Vec<_>>();
        assert_eq!(logits.len(), 21 * 8);
        for anchor in logits.as_chunks::<8>().0 {
            assert!(anchor[4..].iter().all(|value| (value - prior).abs() < 1e-5));
        }
    }
    #[test]
    fn predictions_drop_boxes_clamped_to_zero_area() {
        let device = Device::flex();
        // Two confident anchors: the first box lies inside the image, the second right of it.
        let output = YoloOutput {
            logits: Tensor::from_data([[[0.5f32, 0.5, 0.0, 0.0, 5.0, 5.0]; 2]], &device),
            grid: Tensor::from_data([[[0.0f32, 0.0], [3.0, 0.0]]], &device),
            scale: Tensor::from_data([[[0.5f32, 0.5]; 2]], &device),
        };
        let (_, detections) = output
            .prediction(&DetectionOptions {
                confidence_threshold: 0.5,
                iou_threshold: 0.45,
                max_detections: 10,
            })
            .unwrap();
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].len(), 1);
        let kept = &detections[0][0];
        for (actual, expected) in [kept.x_min, kept.y_min, kept.x_max, kept.y_max]
            .into_iter()
            .zip([0.0, 0.0, 0.5, 0.5])
        {
            assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
        }
    }
    #[test]
    fn loss_scores_objectness_everywhere_and_box_and_class_at_the_assigned_anchor() {
        let device = Device::flex();
        // Anchor 0 is centred in the target and decodes to its top half (IoU 0.5). Anchor 1 is
        // centred outside the box but within the centre radius, so SimOTA assigns anchor 0 alone.
        let logits = [
            [0.5f32, 0.25, 0.0, -std::f32::consts::LN_2, 2.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
        ];
        let assert_loss = |targets: Vec<BoundingBox>, expected: f32| {
            let output = YoloOutput {
                logits: Tensor::from_data([logits], &device),
                grid: Tensor::from_data([[[0.0f32, 0.0], [1.0, 0.0]]], &device),
                scale: Tensor::from_data([[[0.5f32, 0.5]; 2]], &device),
            };
            let values = output
                .loss(&[targets])
                .unwrap()
                .into_data()
                .iter::<f32>()
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 1);
            let actual = values[0];
            assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
        };
        let softplus = |x: f32| (1.0 + x.exp()).ln();
        assert_loss(Vec::new(), softplus(2.0) + softplus(-1.0));
        let target = BoundingBox {
            class_id: 0,
            x_min: 0.0,
            y_min: 0.0,
            x_max: 0.5,
            y_max: 0.5,
        };
        // Box term 1 - 0.5^2 at weight 5; the class target 0.5 (the IoU) costs ln 2 at logit 0.
        assert_loss(
            vec![target],
            softplus(-2.0) + softplus(-1.0) + 5.0 * 0.75 + std::f32::consts::LN_2,
        );
    }
}
