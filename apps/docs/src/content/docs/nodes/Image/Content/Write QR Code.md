---
title: Write QR Code
description: Encodes text into a QR code and outputs it as a NodeImage.
---

## Purpose of the Node
This node generates a QR code image from input text and outputs it as a NodeImage object.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| data | Text to encode | String | |
| scale | Pixels per QR module | Integer | 8 |
| margin | Quiet zone in modules | Integer | 4 |
| End | Done with the Execution | Execution | N/A |
| image_out | QR code image | Struct | NodeImage |
