---
title: Is Bit of Type
description: Checks if the Bit is of the specified type and branches the execution flow accordingly.
---

## Purpose of the Node
The Is Bit of Type node checks if a given Bit is of a specified type. Based on the result, it branches the execution flow into either the "Yes" or "No" output, allowing for conditional execution in visual scripts.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Normal |
| bit | Input Bit | Bit | Struct |
| bit_type | Type to check (e.g., "Llm", "Vlm") | String | Map |
| bit_out | Output Bit | Output Bit | Struct |
| Yes | Execution if Bit is of the specified type | Execution | Normal |
| No | Execution if Bit is not of the specified type | Execution | Normal |