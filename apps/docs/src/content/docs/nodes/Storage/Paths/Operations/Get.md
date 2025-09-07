---
title: File Get
description: Reads all bytes from a file
---

## Purpose of the Node
The File Get node is designed to read all bytes from a specified file path. It takes in a FlowPath as input and outputs the bytes as an array.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Struct | FlowPath |
| Path | The path to the file | FlowPath | Struct |
| End | Marks the end of the execution if the operation is successful | Struct | N/A |
| Bytes | Outputs the bytes read from the file | Byte | Array |
| Failed | Indicates if the operation failed | Execution | N/A |