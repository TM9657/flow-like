---
title: Set System Message
description: Sets the system prompt message in a ChatHistory
---

## Purpose of the Node
This node is used to update the system prompt message in a given `ChatHistory`. It takes the existing history, removes any existing system messages, and inserts the new system prompt at the beginning of the history.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Struct | Normal |
| **History** | ChatHistory | Struct | Array |
| **Message** | System Prompt Message | String | Map |
| **End** | Done with the Execution | Struct | Normal |
| **History** | Updated ChatHistory | Struct | Array |