---
title: InvokeLLM
description: Invokes a language model using the provided model and history. Generates streaming output and the final result.
---

## Purpose of the Node
The InvokeLLM node is designed to invoke a language model using the provided model and history. It handles streaming output by triggering connected nodes on each chunk and generates the final result when the model invocation is complete.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Triggers the model invocation | Execution | Normal |
| Model | The model to be used for the invocation | Struct | Bit |
| History | The chat history to be used for the invocation | Struct | History |
| On Stream | Triggers on streaming output | Execution | Normal |
| Chunk | The current streaming chunk | Struct | ResponseChunk |
| Done | Triggers when the model invocation is complete | Execution | Normal |
| Result | The resulting model output | Struct | Response |