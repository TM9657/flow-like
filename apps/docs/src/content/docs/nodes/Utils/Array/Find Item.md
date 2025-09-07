---
title: Find Item in Array
description: A node that finds the index of an item in an array and whether the item was found.
---

## Purpose of the Node
This node searches through a given array to find the index of a specified item. It outputs the index and a boolean indicating whether the item was found.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| In | Start the execution of the node | Struct | Execution |
| Array | The array in which to search for the item | Array | Generic |
| Item | The item to find in the array | Generic | Normal |
| Out | End the execution of the node | Struct | Execution |
| Index | The index of the found item (-1 if not found) | Integer | Normal |
| Found | Boolean indicating if the item was found | Boolean | Normal |