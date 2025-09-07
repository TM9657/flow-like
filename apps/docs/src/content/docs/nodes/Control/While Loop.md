---
title: While Loop
description: A node that loops downstream execution in a while loop based on a condition.
---

## Purpose of the Node
The While Loop node allows for looping downstream execution in a script based on a condition. It can iterate a specified number of times or until the condition is no longer met, and it provides outputs for the current iteration index and when the loop terminates.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Triggers the start of the while loop | Trigger Pin | Execution |
| Condition | Boolean value that determines whether the loop continues | Boolean | Normal |
| Max | Maximum number of iterations before the loop terminates | Integer | Normal |
| Downstream Execution | Propagation of downstream execution | Downstream execution propagation | Execution |
| Iter | Current iteration index | Current iteration index | Integer |
| Done | Executes once the loop terminates | Executed once the loop terminates | Execution |