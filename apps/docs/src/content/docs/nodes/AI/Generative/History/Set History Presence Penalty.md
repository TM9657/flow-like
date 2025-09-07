---
title: Set History Presence Penalty
description: Sets the presence_penalty attribute in a ChatHistory.
---

## Purpose of the Node
The Set History Presence Penalty node updates the presence penalty value in a provided ChatHistory. This is useful for fine-tuning the behavior of language models by adjusting the penalty for generating tokens that are similar to those in the history.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the execution of the node. | Execution | - |
| **History** | The ChatHistory to which the presence penalty will be applied. | Struct | History |
| **Presence Penalty** | The presence penalty value to be set, ranging from 0.0 to 1.0. | Float | - |
| **End** | Indicates the completion of the node's execution. | Execution | - |
| **History** | The updated ChatHistory with the presence penalty applied. | Struct | History |