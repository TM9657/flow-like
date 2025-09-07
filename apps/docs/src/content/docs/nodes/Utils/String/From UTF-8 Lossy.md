---
title: From UTF-8 Lossy
description: Converts a byte array to a string using the UTF-8 lossy strategy.
---

## Purpose of the Node
The From UTF-8 Lossy node takes a byte array as input and converts it to a string using the UTF-8 lossy strategy. This strategy is useful when you need to handle invalid UTF-8 sequences by replacing them with replacement characters.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| bytes | The byte array to be converted | Struct | Array |
| string | The resulting string after conversion | String | Map |