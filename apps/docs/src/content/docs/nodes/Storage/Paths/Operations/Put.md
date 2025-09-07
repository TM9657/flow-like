---
title: Put
description: Writes bytes to a file
---

## Purpose of the Node
This node writes a specified array of bytes to a file at a given path. It takes in the execution trigger, the path to the file, and the bytes to write. Upon successful completion, it outputs the execution; otherwise, it outputs a failure message.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Struct | Execution |
| Path | The path to the file | Struct | FlowPath |
| Bytes | The bytes to write to the file | Array | Byte |
| End | Done with the Execution | Struct | Execution |
| Failed | Failed to write to the file | Struct | Execution |