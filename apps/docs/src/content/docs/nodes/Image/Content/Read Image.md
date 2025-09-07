---
title: Read Image Path
description: Reads an image from a specified path and outputs the image object.
---

## Purpose of the Node
The Read Image Path node reads an image from the specified file path and outputs the image object. It optionally applies the EXIF orientation to the image before outputting it.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Exec | N/A |
| path | The path to the image file | Path | Struct |
| apply_exif | Whether to apply the EXIF orientation to the image | Boolean | Map |
| End | Outputs the execution completion | Exec | N/A |
| image_out | Outputs the image object | Image | Struct |