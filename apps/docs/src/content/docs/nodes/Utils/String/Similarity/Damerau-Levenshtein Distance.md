---
title: Damerau-Levenshtein Distance
description: Calculates the Damerau-Levenshtein distance between two strings
---

## Purpose of the Node
The Damerau-Levenshtein Distance node calculates the similarity between two strings using the Damerau-Levenshtein algorithm. This algorithm is an extension of the Levenshtein distance that allows for transpositions (swapping of adjacent characters) in addition to insertions, deletions, and substitutions.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Input - First String | String | String |
| Start | Input - Second String | String | String |
| Start | Input - Normalize | Boolean | Boolean |
| End | Output - Distance | Float | Float |