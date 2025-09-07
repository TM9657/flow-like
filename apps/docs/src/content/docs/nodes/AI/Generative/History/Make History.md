---
title: Make History
description: Creates a ChatHistory struct using the provided model name.
---

## Purpose of the Node
The Make History node is used to create a new ChatHistory object with a specified model name. It takes the model name as input and outputs the resulting ChatHistory.

## Pins
| Pin Name    | Pin Description             | Pin Type | Value Type |
|:-------------:|:----------------------------|:---------|:-----------|
| model_name    | The name of the model to create the history for | String | String |
| history       | The created ChatHistory object | Struct | Array |