---
title: Path From Storage Dir
description: Converts the storage directory to a Path
---

## Purpose of the Node
This node converts the storage directory to a `FlowPath` object. It takes an input to initiate execution and provides an output to indicate completion. It also outputs the path if successful.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Exec | N/A |
| Path | Output Path | Struct | FlowPath |
| Node Scope | Is this node in the node scope? | Boolean | False |
| End | Done with the Execution | Exec | N/A |
| Failed | Not possible, for example on server, certain directories are not accessible | Exec | N/A |