---
title: Get Struct Field
description: Fetches a field from a struct.
---

## Purpose of the Node
The Get Struct Field node fetches a specified field from an input struct and outputs the value of that field. It also indicates whether the field was found in the struct.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| struct | Struct Output | Struct | N/A |
| field | Field to get | String | N/A |
| value | Value of the Struct | Generic | N/A |
| found? | Indicates if the value was found | Boolean | N/A |