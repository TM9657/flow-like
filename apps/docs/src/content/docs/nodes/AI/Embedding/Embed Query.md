---
title: Embed Query
description: A node that embeds a query string using a loaded model.
---

## Purpose of the Node
The Embed Query node is designed to take a query string and an embedding model as inputs, and output an embedding vector. It supports both text and image models.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | - |
| query_string | The string to embed | String | - |
| model | The embedding model | Struct | CachedEmbeddingModel |
| End | Done with the Execution | Execution | - |
| vector | The embedding vector | Float | Array |
| Failed | Failed to embed the query | Execution | - |