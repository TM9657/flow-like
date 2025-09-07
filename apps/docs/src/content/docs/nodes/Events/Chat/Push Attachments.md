---
title: Push Attachments
description: A node that pushes attachments to a chat.
---

## Purpose of the Node
This node is used to push attachments to a chat by adding them to a cached chat response and streaming a partial response containing these attachments.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the execution of the node. | Execution | N/A |
| **Attachments** | The attachments to be pushed to the chat. | Struct | Array |
| **End** | Indicates the completion of the node's execution. | Execution | N/A |