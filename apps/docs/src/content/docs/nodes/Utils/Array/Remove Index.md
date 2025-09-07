---
title: Remove Array Index
description: Removes an element from an array at a specific index.
---

## Purpose of the Node
The Remove Array Index node removes an element from an array at a specified index. It provides outputs for the adjusted array, a failure execution pin if the removal fails, and an output execution pin to indicate successful removal.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Index** | Index to remove | Number | Integer |
| **Array In** | Your Array | Array | Array |
| **Failed Removal** | Triggered if the Removal failed | Execution | Normal |
| **Exec In** | Start | Execution | Normal |
| **Array Out** | Adjusted Array | Array | Array |
| **Exec Out** | End | Execution | Normal |