---
title: Set History Top P
description: Sets the top_p attribute in a ChatHistory
---

## Purpose of the Node
This node updates the top_p attribute in a ChatHistory. It takes the current ChatHistory and a top_p value as inputs and outputs the updated ChatHistory.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Execution | N/A |
| History | The ChatHistory to be updated | Struct | History |
| Top P | The top_p value to set | Float | N/A |
| End | Indicates the completion of the execution | Execution | N/A |
| History | The updated ChatHistory | Struct | History |