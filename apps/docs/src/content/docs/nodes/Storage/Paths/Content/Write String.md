---
title: Write String
description: Writes a string to a file
---

## Purpose of the Node
The Write String node is designed to write a string content to a specified file path. It initiates execution, receives the path and content as inputs, and outputs completion or failure based on the operation's success.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Input, Initiates Execution | Execution | N/A |
| **Path** | The file path to write to | Struct | FlowPath |
| **Content** | The string content to write | String | N/A |
| **End** | Output, Done with the Execution | Execution | N/A |
| **Failed** | Output, Triggered if writing the file fails | Execution | N/A |