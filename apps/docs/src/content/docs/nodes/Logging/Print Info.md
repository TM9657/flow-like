---
title: Info
description: Prints debugging information to the console and optionally shows a toast message to the user.
---

## Purpose of the Node
This node logs debugging information and optionally displays a toast notification to the user. It takes an input message and a boolean to determine whether to show a toast message.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger Pin, activates the node | Struct | Normal |
| Message | The message to log | String | Normal |
| On Screen? | Whether to show a toast message | Boolean | Normal |
| End | Output Pin, activates the next node | Struct | Normal |