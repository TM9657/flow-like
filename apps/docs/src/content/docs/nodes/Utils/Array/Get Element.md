---
title: Get Element from Array
description: Retrieves an element from an array at a specified index.
---

## Purpose of the Node
The Get Element from Array node retrieves an element from an array at a specified index and outputs the element along with a success flag indicating whether the retrieval was successful.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| array_in | Array to retrieve an element from | Struct | Array |
| index | Index of the element to get | Struct | Integer |
| element | Element at the specified index | Struct | Normal |
| success | Was the get successful? | Struct | Boolean |