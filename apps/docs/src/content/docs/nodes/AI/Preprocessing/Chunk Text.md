---
title: Chunk Text
description: A node that chunks the text into smaller pieces for efficient embedding.
---

## Purpose of the Node
The `Chunk Text` node is designed to efficiently break down text into smaller chunks, which can be useful for processing large text documents or preparing text for embedding models.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Execution | Normal |
| **Text** | The string to embed | String | Normal |
| **Model** | The embedding model | Struct | CachedEmbeddingModel |
| **Capacity** | Chunk Capacity | Integer | Normal |
| **Overlap** | Overlap between Chunks | Integer | Normal |
| **Markdown** | Use Markdown Splitter? | Boolean | Normal |
| **End** | Done with the Execution | Execution | Normal |
| **Chunks** | The embedding vector | String | Array |
| **Failed** | Failed to embed the query | Execution | Normal |