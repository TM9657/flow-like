---
title: Rename
description: Renames a file from a source path to a destination path.
---

## Purpose of the Node
This node is designed to rename a file located at a specified source path to a new path. It can also handle the case where the destination file already exists, either by overwriting it or by skipping the operation.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Struct | None |
| From | Source FlowPath | Struct | FlowPath |
| To | Destination FlowPath | Struct | FlowPath |
| Overwrite | Should the destination file be overwritten? | Boolean | false |
| End | Done with the Execution | Struct | None |
| Failed | Failed to move the file | Struct | None |