---
title: Color Convert Node
description: A node to convert the color or pixel type of an image.
---

## Purpose of the Node
The Color Convert Node allows you to convert an image to a different color type such as RGB, RGBA, Luma, or LumaA. You can choose to use a reference of the image, meaning the original image will be transformed, or you can work with a copy.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Execution |
| Image In | Input Image | Struct | NodeImage |
| Pixel Type | Target Pixel Type | String | String (Valid values: "RGB", "RGBA", "Luma", "LumaA") |
| Use Reference | Use Reference of the image, transforming the original instead of a copy | Boolean | Boolean |
| End | Done with the Execution | Execution | Execution |
| Image Out | Image with Target Color/Pixel Type | Struct | NodeImage |