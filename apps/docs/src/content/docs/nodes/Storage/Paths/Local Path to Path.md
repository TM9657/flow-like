---
title: PathBuf to Path
description: Converts a PathBuf to a Path
---

## Purpose of the Node
The PathBuf to Path node is used to convert a PathBuf to a Path, providing flexibility in handling file and directory paths within your visual scripting environment.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Execution | N/A |
| Local Path | Input PathBuf to be converted to a Path | PathBuf | N/A |
| Path | Output Path derived from the input PathBuf | Struct | FlowPath |
| Failed | Indicates that the conversion could not be completed (e.g., due to server restrictions) | Execution | N/A |
| End | Marks the end of the node's execution | Execution | N/A |