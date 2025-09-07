---
title: Set History Max Tokens
description: Sets the `max_tokens` attribute in a ChatHistory
---

## Purpose of the Node
This node sets the `max_tokens` attribute in a ChatHistory by receiving the ChatHistory and the desired max tokens value as inputs.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Execution | N/A |
| **History** | ChatHistory | Struct | Array |
| **Max Tokens** | Max Tokens Value | Integer | N/A |
| **End** | Done with the Execution | Execution | N/A |
| **History** | Updated ChatHistory | Struct | Array |