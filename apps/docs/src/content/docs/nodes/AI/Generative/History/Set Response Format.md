---
title: Set History Response Format
description: Sets the response_format attribute in a ChatHistory
---

## Purpose of the Node
This node is used to update the response_format attribute in a ChatHistory object. It accepts a ChatHistory and a response format, which can be either a string or an object, and returns the updated ChatHistory.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| History | ChatHistory | Struct | History |
| Response Format | Response Format Value | Generic | String / Object |
| End | Done with the Execution | Execution | N/A |
| History Out | Updated ChatHistory | Struct | History |