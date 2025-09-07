---
title: List Paths
description: Lists all paths in a directory, either recursively or not, based on the provided prefix.
---

## Purpose of the Node
The List Paths node initiates the execution of a process to list all paths within a specified directory, either recursively or not, and outputs the paths if successful or an error if the process fails.

## Pins
The List Paths node has input and output pins to control and receive data during the execution.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Struct | Execution |
| **Prefix** | The directory path and prefix to start listing from | Struct | FlowPath |
| **Recursive** | Determines whether to list paths recursively | Boolean | Normal |
| **End** | Marks the completion of the execution | Struct | Execution |
| **Paths** | Outputs the list of paths found | Struct | Array of FlowPath |
| **Failed** | Outputs an execution if the paths listing fails | Struct | Execution |