---
title: Get Range
description: Reads a range of bytes from a file
---

## Purpose of the Node
This node reads a specified range of bytes from a file and outputs the result. It is useful for extracting specific parts of a file without loading the entire file into memory.

## Pins
This node has several input and output pins that facilitate the process of reading a byte range from a file.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Normal |
| Path | FlowPath | String | FlowPath |
| From | Start of the Range | Integer | Number |
| To | End of the Range | Integer | Number |
| End | Done with the Execution | Execution | Normal |
| Failed | Failed to get the range | Execution | Normal |
| Bytes | Output Bytes | Byte | Array |