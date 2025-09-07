---
title: Contrast Image Node
description: Adjusts the contrast of an input image.
---

## Purpose of the Node
The Contrast Image Node is used to adjust the contrast of an input image. It takes an image and a contrast value as inputs and outputs the adjusted image.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | - |
| image_in | Input Image | Struct | NodeImage |
| contrast | Contrast Value | Float | - |
| use_ref | Use Reference of the Image | Boolean | - |
| End | Done with Execution | Execution | - |
| image_out | Image with Applied Contrast | Struct | NodeImage |