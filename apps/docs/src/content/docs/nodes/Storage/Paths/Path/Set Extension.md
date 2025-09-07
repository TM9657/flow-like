---
title: Set Extension
description: Sets the file extension of a provided path.
---

## Purpose of the Node
This node is used to modify the file extension of a given path. It takes a path and a new file extension as input, and outputs the modified path.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Input, initiate the execution of the node. | Execution | Normal |
| Path | Input, the path to modify the extension of. | Struct | FlowPath |
| Extension | Input, the new file extension to set. | String | Normal |
| End | Output, indicates the end of the execution. | Execution | Normal |
| Path | Output, the path with the modified extension. | Struct | FlowPath |