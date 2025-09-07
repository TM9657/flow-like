---
title: Sequence
description: A Sequence node executes its connected nodes in a specific order, one after the other.
---

## Purpose of the Node
The Sequence node is designed to execute a series of nodes sequentially. Each node connected to its output pin will be triggered in the order they are connected. This is useful for creating ordered workflows where the outcome of each node must be successful before the next one starts.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Input pin that triggers the sequence execution | Trigger Pin | Execution |
| End | Output pin that marks the end of the sequence execution | Output Pin | Execution |