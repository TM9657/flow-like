---
title: Embed Document
description: Embeds a document string using a loaded model
---

## Purpose of the Node
The Embed Document node takes a query string and a model, then embeds the document string using the specified model. The output includes the embedding vector or an error if the embedding fails.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Struct | Normal |
| Query String | The string to embed | String | Map |
| Model | The embedding model | Struct | Array |
| End | Done with the Execution | Struct | Normal |
| Vector | The embedding vector | Float | Set |
| Failed | Failed to embed the query | Struct | Normal |