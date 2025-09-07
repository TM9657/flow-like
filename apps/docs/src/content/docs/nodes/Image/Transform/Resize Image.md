---
title: Resize Image
description: Resizes an image to the specified dimensions with an optional reference mode and filter algorithm.
---

## Purpose of the Node
The Resize Image node resizes an input image to a specified target width and height using various resize modes and filter algorithms. It also supports using a reference mode to transform the original image instead of a copy.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| Image In | Image object to be resized | Struct | NodeImage |
| Use Reference | Use Reference of the image, transforming the original instead of a copy | Boolean | True |
| Mode | Resize Mode | String | "keep_aspect", "exact", or "to_fill" |
| Filter | Resize Filter Algorithm | String | "Nearest", "Triangle", "CatmullRom", "Gaussian", or "Lanczos3" |
| Width In | Resized Image Target Width | Integer | 512 |
| Height In | Resized Image Target Height | Integer | 512 |
| End | Done with the Execution | Execution | N/A |
| Image Out | Resized Image object | Struct | NodeImage |
| Width Out | Resized Image Result Width | Integer | N/A |
| Height Out | Resized Image Result Height | Integer | N/A |