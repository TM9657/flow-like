---
title: Get Token
description: Extracts the token from a ResponseChunk
---

## Purpose of the Node
The Get Token node extracts the token from a provided ResponseChunk. This is useful in scenarios where you need to process or utilize the token extracted from an AI generative response.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Chunk | The response chunk from which to extract the token | Struct | ResponseChunk |
| Token | The token extracted from the response chunk | String | - |