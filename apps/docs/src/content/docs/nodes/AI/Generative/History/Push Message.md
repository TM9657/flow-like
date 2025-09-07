---
title: Push History Message
description: Adds a message to a ChatHistory
---

## Purpose of the Node
This node is designed to add a message to a chat history. It takes in an existing chat history and a new message, and outputs the updated chat history.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Normal |
| History | ChatHistory | Struct | History |
| Message | Message to add | Struct | HistoryMessage |
| End | Done with the Execution | Execution | Normal |
| History | Updated ChatHistory | Struct | History |