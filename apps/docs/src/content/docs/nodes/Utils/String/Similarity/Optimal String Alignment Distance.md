---
title: Optimal String Alignment Distance
description: A node that calculates the Optimal String Alignment distance between two strings.
---

## Purpose of the Node
This node is designed to calculate the Optimal String Alignment distance between two input strings. It utilizes the `strsim::osa_distance` function to determine the minimal number of single-character edits required to change one string into the other.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **string1** | The first string for comparison. | Input | String |
| **string2** | The second string for comparison. | Input | String |
| **distance** | The calculated Optimal String Alignment distance. | Output | Float |