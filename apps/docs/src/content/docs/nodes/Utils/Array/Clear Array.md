---
title: Clear Array
description: A Clear Array node that removes all elements from an array.
---

## Purpose of the Node
The Clear Array node is designed to clear all elements from an array by setting it to an empty array. It takes an input array and an execution signal, and outputs the cleared array along with an execution signal.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to start the node | Execution | - |
| Array | The array to be cleared | Generic | Array |
| End | Trigger indicating the node has completed | Execution | - |
| Array Out | The cleared array | Generic | Array |