---
title: From Messages
description: Creates a Chat History from Messages
---

## Purpose of the Node
The From Messages node takes a list of chat messages and a model name as input, and outputs a structured chat history. This node is useful for integrating chat histories into generative AI applications.

## Pins
The From Messages node has the following pins:

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Model Name | The name of the AI model used in the chat. | String | Map |
| Messages | An array of chat messages. Each message is a struct containing the sender's name and the message content. | Struct | Array |
| History | The resulting chat history. | Struct | Normal |