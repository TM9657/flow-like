---
title: Read to Bytes
description: Reads the content of a file to bytes.
---

## Purpose of the Node
The Read to Bytes node reads the content of a specified file and outputs the content as bytes. It is useful for handling file reading operations in a visual scripting environment.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| Path | Path to the file | Struct | FlowPath |
| Content | The content of the file as bytes | Byte | Array |
| Failed | Triggered if reading the file fails | Execution | N/A |