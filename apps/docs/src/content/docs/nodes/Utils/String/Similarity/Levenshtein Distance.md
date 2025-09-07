---
title: Levenshtein Distance
description: A node that calculates the Levenshtein distance between two strings.
---

## Purpose of the Node
This node calculates the Levenshtein distance between two strings. The Levenshtein distance is a measure of the similarity between two strings, defined as the minimum number of single-character edits (insertions, deletions, or substitutions) required to change one string into the other. Optionally, the distance can be normalized to a value between 0 and 1.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Connect this pin to start the node's execution | Start | - |
| string1 | First String | Input | String |
| string2 | Second String | Input | String |
| normalize | Normalize the Distance | Input | Boolean |
| End | Connect this pin to receive the node's output | End | Float |
| distance | Levenshtein Distance | Output | Float |