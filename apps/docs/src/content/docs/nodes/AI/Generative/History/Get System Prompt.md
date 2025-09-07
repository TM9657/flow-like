---
title: Get System Prompt
description: Retrieves the system prompt from a ChatHistory
---

## Purpose of the Node
The Get System Prompt node retrieves the system prompt message from a provided ChatHistory. This is useful for scenarios where you need to process or analyze the system instructions given in a chat session.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| history | Input chat history to search for the system prompt | Struct | ChatHistory |
| system_prompt | Output: The found system prompt message | Struct | HistoryMessage |
| success | Output: Boolean indicating if the system prompt was found | Boolean | Boolean |