---
title: ErrorNode
description: A node that logs or displays an error message.
---

## Purpose of the Node
The ErrorNode is designed to log an error message to the console and optionally display a toast notification to the user. It is useful for handling errors in your visual scripting workflow.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger the execution of the node | Input | Execution |
| Message | The error message to log and optionally display | String | Map |
| On Screen? | Determines if a toast notification should be shown | Boolean | Normal |
| End | Continue the flow if the error handling is successful | Output | Execution |