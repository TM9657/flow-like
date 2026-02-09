---
title: Read QR-/Barcode
description: Detects and decodes QR codes and barcodes from images.
---

## Purpose of the Node
This node reads and decodes QR codes and barcodes from an input image. It can detect multiple codes of various types and return the results.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| image_in | Image object | Struct | NodeImage |
| filter | Filter for Certain Code Type | Boolean | false |
| Results | Detected/Decoded Codes | Array of Struct | Barcode |
| End | Done with the Execution | Execution | N/A |