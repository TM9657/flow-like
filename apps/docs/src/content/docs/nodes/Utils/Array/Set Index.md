---
title: Set Index Array
description: Sets an element at a specific index in an array.
---

## Purpose of the Node
This node takes an input array, an index, and a value, then sets the element at the specified index in the array with the given value. If the index is out of bounds, it triggers a failure output.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to start the process | Struct | Array |
| Array | The input array where the value will be set | Array | Generic |
| Index | The index at which the value should be set | Number | Integer |
| Value | The value to be set at the specified index | Generic | Generic |
| End | Trigger indicating the successful execution | Struct | Array |
| Array | The adjusted array with the value set | Array | Generic |
| Failed Setting | Trigger if the operation fails due to an out-of-bounds index | Struct | Array |