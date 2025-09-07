---
title: PathFromCacheDir
description: Converts the cache directory to a Path
---

## Purpose of the Node
This node retrieves the cache directory path and outputs it as a `FlowPath` object. It also allows you to specify whether the node is within the node scope.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | - |
| End | Done with the Execution | Execution | - |
| path | Output Path | Struct | FlowPath |
| node_scope | Is this node in the node scope? | Boolean | - |
| failed | Not possible, for example on server, certain directories are not accessible | Execution | - |