---
title: Load Model
description: A node that loads an embedding model from a Bit.
---

## Purpose of the Node
The Load Model node is designed to load an embedding model from a specified Bit. It checks if the Bit type is either Embedding or ImageEmbedding before proceeding with the model loading process.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the execution of the node. | Execution | N/A |
| **Model Bit** | The Bit that contains the model. | Struct | Bit |
| **Model** | The loaded embedding model. | Struct | CachedEmbeddingModel |
| **Failed Loading** | Indicates if the model loading was unsuccessful. | Execution | N/A |