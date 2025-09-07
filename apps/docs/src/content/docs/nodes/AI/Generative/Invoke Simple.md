---
title: InvokeLLMSimpleNode
description: A simple node that invokes an LLM and processes the streaming output.
---

## Purpose of the Node
The `InvokeLLMSimpleNode` node is designed to invoke an LLM (Large Language Model) with a given system prompt and user prompt. It handles the streaming output and triggers connected nodes accordingly.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger pin to start the node execution | Execution | N/A |
| Model | Model to be invoked | Struct | Bit |
| System Prompt | System prompt for the LLM | String | N/A |
| Prompt | User prompt for the LLM | String | N/A |
| On Stream | Trigger pin activated on streaming output | Execution | N/A |
| Token | Streaming token from the LLM | String | N/A |
| Done | Trigger pin activated once the execution is done | Execution | N/A |
| Result | Resulting model output | String | N/A |