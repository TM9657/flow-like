---
title: Delay
description: Delays execution for a specified amount of time
---

## Purpose of the Node
The Delay node is used to introduce a pause in the execution flow. It waits for a specified duration before continuing, which can be useful for synchronizing events or simulating delays in a workflow.

## Pins
- **Time (ms):** The delay time in milliseconds. The default value is set to 1000.0 milliseconds (1 second).

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | The input pin that triggers the delay. | Execution | - |
| **Time (ms)** | Specifies the delay time in milliseconds. | Float | - |
| **End** | The output pin that is activated after the delay has completed. | Execution | - |