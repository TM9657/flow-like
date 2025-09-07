---
title: Root Integer
description: A node that calculates the nth root of an integer.
---

## Purpose of the Node
This node takes an integer and calculates its nth root. It handles errors if the degree is not a positive integer.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Radicand** | The integer to take the root of | Struct | Array |
| **Degree** | The degree of the root | String | Map |
| **Root** | Result of the root calculation | Number | Float |