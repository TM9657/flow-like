---
title: Set History Temperature
description: Sets the temperature attribute in a ChatHistory
---

## Purpose of the Node
The Set History Temperature node updates the temperature attribute in a given ChatHistory. This can be useful for adjusting the randomness or creativity of generated responses in AI-based conversations.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Execution | - |
| **History** | ChatHistory | Struct | History |
| **Temperature** | Temperature Value | Float | 0.0 to 2.0 |
| **End** | Done with the Execution | Execution | - |
| **History** | Updated ChatHistory | Struct | History |