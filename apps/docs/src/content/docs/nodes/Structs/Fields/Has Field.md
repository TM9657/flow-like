---
title: Has Field
description: Checks if a field exists in a struct
---

## Purpose of the Node
The Has Field node is used to determine whether a specific field exists within a given struct. This node evaluates the presence of a field in the struct and outputs a boolean value indicating the result.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| struct | Struct Output | Struct | Array |
| field | Field to get | String | Map |
| found | Indicates if the value was found | Boolean | Normal |