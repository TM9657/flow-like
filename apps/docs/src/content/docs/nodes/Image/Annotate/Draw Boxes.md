---
title: Draw Bounding Boxes
description: This node draws bounding boxes on an input image using provided bounding box coordinates and annotations. It can operate on a reference of the image or create a copy, providing flexibility in how the image is modified.
---

## Purpose of the Node
The `Draw Bounding Boxes` node is designed to annotate images with bounding boxes and labels based on detected objects. It accepts an input image and a list of bounding boxes, and outputs the annotated image. The node can operate on either a reference to the original image or create a new copy, allowing users to choose the desired behavior.

## Pins

| Pin Name      | Pin Description                                                  | Pin Type | Value Type   |
|---------------|------------------------------------------------------------------|----------|--------------|
| **Start**     | Initiate Execution.                                            | Exec     | None         |
| **Image In**  | The input image to be annotated.                                 | Struct   | `NodeImage`  |
| **Boxes**     | The bounding boxes to be drawn on the image.                      | Struct   | `BoundingBox` array |
| **Use Reference** | Whether to use a reference of the image or create a copy. | Boolean  | Boolean      |
| **End**       | Output Execution Completion.                                   | Exec     | None         |
| **Image Out** | The image with bounding boxes drawn on it.                         | Struct   | `NodeImage`  |