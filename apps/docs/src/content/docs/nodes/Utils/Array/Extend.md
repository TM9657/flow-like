---
title: Extend Array
description: This node extends one array with another array.
---

## Purpose of the Node
This node appends an array to another array. It takes two input arrays and outputs the extended array.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| In | The trigger for the node execution. | Struct | Array |
| Array | The array to be extended. | String | Map |
| Values | The values to be appended to the array. | Number | Normal |
| Out | The trigger indicating the end of the execution. | Float | Set |
| Array | The extended array. | Struct | Array |