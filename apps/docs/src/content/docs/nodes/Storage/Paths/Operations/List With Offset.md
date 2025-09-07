---
title: List With Offset
description: Lists paths in a directory with offset and limit.
---

## Purpose of the Node
The List With Offset node retrieves a list of paths from a specified directory, applying an offset to determine the starting point of the list. This node is useful for handling pagination scenarios.

## Pins
The List With Offset node has input and output pins to control the execution flow and manage data passing.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Exec | N/A |
| **Prefix** | Directory prefix to list paths from | Struct | FlowPath |
| **Offset** | Offset to start listing from | Integer | N/A |
| **End** | Done with the Execution | Exec | N/A |
| **Failed** | Failed to list the paths | Exec | N/A |
| **Paths** | Output Paths | Struct | Array of FlowPath |