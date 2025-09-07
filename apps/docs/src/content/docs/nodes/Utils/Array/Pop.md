---
title: PopArray
description: Removes and returns the last element of an array
---

## Purpose of the Node
The PopArray node is designed to remove and return the last element from an input array. It provides outputs for the adjusted array, the popped value, and an execution output to indicate success or failure.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to start the operation | Struct | Execution |
| array_in | Input array from which the last element will be removed | String | Array |
| End | Execution output indicating successful operation | Struct | Execution |
| array_out | Output array with the last element removed | String | Array |
| value | Popped value from the array | Number | Normal |
| failed | Execution output indicating failure | Struct | Execution |