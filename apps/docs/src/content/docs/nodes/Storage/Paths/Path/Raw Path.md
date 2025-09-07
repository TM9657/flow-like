---
title: Raw Path
description: Retrieves the raw path string from a FlowPath node.
---

## Purpose of the Node
This node is designed to extract and return the raw path string from a `FlowPath` object. It takes a `FlowPath` input and outputs the corresponding raw path string as a string.

## Pins
This node has two pins: an input pin to receive a `FlowPath` object and an output pin to provide the raw path string.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Path | The input pin receives a `FlowPath` object. | Struct | FlowPath |
| Raw Path | The output pin provides the raw path string extracted from the `FlowPath` object. | String | String |