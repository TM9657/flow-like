---
title: WarningNode
description: A node that logs a warning message and optionally shows a toast notification.
---

## Purpose of the Node
This node logs a warning message and optionally displays a toast notification to the user. It is useful for alerting users about potential issues or conditions in the flow.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger Pin | Execution | - |
| Message | Print Warning | String | Map |
| On Screen? | Should the user see a toast popping up? | Boolean | Normal |
| End | The flow to follow if the condition is true | Execution | Set |