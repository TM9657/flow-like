---
title: Push Chunk
description: Pushes a response chunk to the chat
---

## Purpose of the Node
The Push Chunk node is used to push a generated chat chunk to the chat system. It takes an input execution and a chunk of data, and then processes this data to update the cached chat response and stream a partial response back to the client.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Struct | Execution |
| **Chunk** | Generated Chat Chunk | Struct | ResponseChunk |
| **End** | Done with the Execution | Struct | Execution |