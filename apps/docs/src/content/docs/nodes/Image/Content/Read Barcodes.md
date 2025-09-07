---
title: Read Barcodes
description: Detects and decodes barcodes from images.
---

## Purpose of the Node
This node is designed to read and decode barcodes from an input image. It can detect multiple barcodes of various types and return the results.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| image_in | Image object | Struct | NodeImage |
| filter | Filter for Certain Code Type | Boolean | false |
| Results | Detected/Decoded Codes | Array of Struct | Barcode |
| End | Done with the Execution | Execution | N/A |