---
title: Make Bounding Box
description: Creates a Bounding Box from input parameters.
---

## Purpose of the Node
This node allows you to create a Bounding Box based on various input parameters, such as the type of definition (xyxy or x1y1wh), class index, score, and coordinates. It also dynamically manages the input pins based on the selected definition type.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Execution | - |
| Definition | Specifies the format of the bounding box definition | String | - |
| Class | The class index for the bounding box | Integer | - |
| Score | The score or confidence of the bounding box | Float | - |
| x1 | The left coordinate of the bounding box for "xyxy" format | Float | - |
| y1 | The top coordinate of the bounding box for "xyxy" format | Float | - |
| x2 | The right coordinate of the bounding box for "xyxy" format | Float | - |
| y2 | The bottom coordinate of the bounding box for "xyxy" format | Float | - |
| w | The width of the bounding box for "x1y1wh" format | Float | - |
| h | The height of the bounding box for "x1y1wh" format | Float | - |
| End | Marks the completion of the node's execution | Execution | - |
| Box | Outputs the created bounding box in the specified format | Struct | BoundingBox |