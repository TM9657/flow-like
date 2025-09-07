---
title: Image Classification
description: Predict classes for images using an ONNX model.
---

## Purpose of the Node
The Image Classification node predicts the class of an input image using an ONNX model. It applies default transformations to the input image such as resizing, center cropping, normalizing, and scaling pixel values to ensure compatibility with the ONNX model. The node also supports optional softmax activation to scale the output logits to probabilities.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | - |
| Model | ONNX Model Session | Struct | NodeOnnxSession |
| Image | Image Object | Struct | NodeImage |
| Mean | Image Mean for Normalization (per channel) | Float | Array |
| Std | Image Standard Deviation for Normalization (per channel) | Float | Array |
| Crop | Center Crop Percentage | Float | - |
| Softmax? | Scale Outputs with Softmax | Boolean | - |
| End | Done with the Execution | Execution | - |
| Predictions | Class Predictions | Struct | Array |