---
title: Path Exists
description: Checks if a path exists in the storage system
---

## Purpose of the Node
The Path Exists node is used to determine if a specified path exists in the storage system. It takes an input path and evaluates whether it exists, then outputs different execution paths based on the result.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiate Execution | Exec | N/A |
| **Path** | FlowPath to check | Struct | FlowPath |
| **Yes** | Execution if path exists | Exec | N/A |
| **No** | Execution if path does not exist | Exec | N/A |