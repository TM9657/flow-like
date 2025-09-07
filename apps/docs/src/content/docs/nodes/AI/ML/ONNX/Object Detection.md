---
title: Object Detection
description: Evaluate ONNX-based Object Detection Models for Images
---

## Purpose of the Node
The Object Detection node evaluates ONNX-based object detection models for images. It takes an input image, an ONNX model session, and confidence and IoU thresholds as inputs, and returns bounding box predictions.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Execution |
| Model | ONNX Model Session | Struct | NodeOnnxSession |
| Image | Image Object | Struct | NodeImage |
| Conf | Confidence Threshold | Float | Normal |
| IoU | Intersection Over Union Threshold for NMS | Float | Normal |
| Max | Maximum Number of Detections | Integer | Normal |
| End | Done with the Execution | Execution | Execution |
| Boxes | Bounding Box Predictions | Struct | BoundingBox (Array) |