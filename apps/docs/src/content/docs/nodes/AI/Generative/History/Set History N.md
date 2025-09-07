---
title: Set History N
description: A node that updates the `n` attribute in a ChatHistory.
---

## Purpose of the Node
The `Set History N` node is used to modify the `n` attribute of a `ChatHistory` object. It takes an input `ChatHistory`, an integer `N`, and outputs the updated `ChatHistory`.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Struct | Execution |
| History | The input `ChatHistory` object | Struct | History |
| N | The integer value to set for the `n` attribute | Integer | Normal |
| End | Indicates the completion of the node's execution | Struct | Execution |
| History | Outputs the updated `ChatHistory` object | Struct | History |