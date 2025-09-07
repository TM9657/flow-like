---
title: Pop History Message
description: Removes and returns the last message from a ChatHistory
---

## Purpose of the Node
This node is designed to remove and return the last message from a given ChatHistory. It takes an input execution signal and a ChatHistory, then outputs the removed message, the updated ChatHistory, or an execution signal indicating that the history was empty.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | - |
| History | ChatHistory | Struct | History |
| End | Done with the Execution | Execution | - |
| History | Updated ChatHistory | Struct | History |
| Message | Removed Message | Struct | HistoryMessage |
| Empty | History was empty | Execution | - |