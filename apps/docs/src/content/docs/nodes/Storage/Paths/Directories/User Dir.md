---
title: PathFromUserDir
description: Converts the user directory to a Path
---

## Purpose of the Node
The PathFromUserDir node retrieves the user's directory and outputs it as a Path. It includes error handling to manage scenarios where the directory conversion might fail, such as on a server where certain directories are not accessible.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| End | Done with the Execution | Execution | N/A |
| Path | Output Path | Struct | FlowPath |
| Node Scope | Is this node in the node scope? | Boolean | true/false |
| Failed | Not possible, for example on server, certain directories are not accessible | Execution | N/A |