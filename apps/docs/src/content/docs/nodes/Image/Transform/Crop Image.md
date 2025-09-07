---
title: Crop Image
description: Crops an image based on a bounding box.
---

## Purpose of the Node
The Crop Image node is used to crop an image object based on the provided bounding box. It can either transform the original image or create a new cropped image, depending on the "Use Reference" input.

## Pins
The Crop Image node has the following pins:

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Execution | Normal |
| image_in | The image object to be cropped | Struct | NodeImage |
| bbox | The bounding box defining the crop area | Struct | BoundingBox |
| use_ref | Determines whether to transform the original image (false) or create a new cropped image (true) | Boolean | Normal |
| End | Indicates the completion of the execution | Execution | Normal |
| image_out | The cropped image object | Struct | NodeImage |