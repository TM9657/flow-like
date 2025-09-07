---
title: Clear History
description: Clears all messages from a ChatHistory
---

## Purpose of the Node
The Clear History node is designed to clear all messages from a given ChatHistory. It accepts an input with the current ChatHistory and outputs a new ChatHistory with all messages removed.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | None |
| History | ChatHistory to be cleared | Struct | History |
| End | Done with the Execution | Execution | None |
| History | Cleared ChatHistory | Struct | History |