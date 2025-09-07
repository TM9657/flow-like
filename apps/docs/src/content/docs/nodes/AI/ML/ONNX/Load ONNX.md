---
title: Load ONNX Node
description: A node to load ONNX Runtime session from a specified path.
---

## Purpose of the Node
The Load ONNX Node is designed to load an ONNX model from a specified path and prepare it for execution. It reads the model file, determines the input and output tensors, and initializes a session with these specifications. This node is particularly useful in scenarios involving AI/ML workflows where ONNX models are utilized.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the execution of the node. | Execution | N/A |
| **Path** | Provides the path to the ONNX file. | Struct | FlowPath |
| **End** | Marks the completion of the node's execution. | Execution | N/A |
| **Model** | Outputs the loaded ONNX model session. | Struct | NodeOnnxSession |