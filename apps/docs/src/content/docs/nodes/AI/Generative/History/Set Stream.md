---
title: Set History Stream
description: Sets the stream attribute in a ChatHistory
---

## Purpose of the Node
This node is designed to update the stream attribute of a ChatHistory. It takes in a ChatHistory and a boolean value representing the stream attribute, and outputs the updated ChatHistory.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node. | Execution | - |
| History | The ChatHistory to be updated. | Struct | History |
| Stream | The boolean value representing the new stream attribute. | Boolean | - |
| End | Signals the end of the node's execution. | Execution | - |
| History | The updated ChatHistory with the new stream attribute. | Struct | History |