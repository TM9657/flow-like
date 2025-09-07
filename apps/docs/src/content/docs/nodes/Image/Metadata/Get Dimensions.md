---
title: Image Dims
description: Retrieves the dimensions (width and height) of an image.
---

## Purpose of the Node
The Image Dims node is used to extract and retrieve the width and height of an image from an input image object. This node is particularly useful when you need to know the dimensions of an image before proceeding with further image processing tasks.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the execution of the node. | Execution | N/A |
| **Image** | The image object whose dimensions are to be retrieved. | Struct | `NodeImage` |
| **End** | Signals the completion of the node's execution. | Execution | N/A |
| **width** | The width of the image. | Integer | N/A |
| **height** | The height of the image. | Integer | N/A |