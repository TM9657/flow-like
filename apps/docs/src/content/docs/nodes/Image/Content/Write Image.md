---
title: Write Image
description: Writes an image to a specified path with options for image type and quality.
---

## Purpose of the Node
The Write Image node allows you to save an image to a specified path. It supports various image formats such as JPEG, PNG, AVIF, BMP, ICO, and more. The node also provides options for encoding quality, compression settings, and encoding speed.

## Pins
The node has several input and output pins. The Start pin initiates the execution, while the End pin indicates the completion of the execution.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Exec | None |
| Image | The image to write to the specified path | Struct | NodeImage |
| Path | The flow path where the image will be saved | Struct | FlowPath |
| Type | The type of the image (e.g., JPEG, PNG) | String | JPEG |
| Quality | The encoding quality for JPEG | Integer | 100 |
| Compression Type | The compression type for PNG (Best, Fast, Default) | String | Default |
| Filter | The filter type for PNG (NoFilter, Sub, Up, Average, Adaptive, Paeth) | String | Adaptive |
| Speed | The encoding speed for GIF and AVIF | Integer | 10 |
| Threads | The number of threads for AVIF encoding | Integer | 1 |
| End | Indicates the completion of the execution | Exec | None |