---
title: Buffered CSV Reader
description: A node that reads a CSV file and processes it in chunks.
---

## Purpose of the Node
The Buffered CSV Reader node is designed to read a CSV file from a specified path and process it in chunks. This is useful for handling large CSV files that might not fit into memory all at once. The node allows you to specify the delimiter and chunk size, making it versatile for various CSV file formats.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the execution of the node | Execution | N/A |
| **CSV** | The path to the CSV file to be read | Struct | FlowPath |
| **Chunk Size** | The number of records per chunk | Integer | N/A |
| **Delimiter** | The delimiter character used in the CSV file | String | N/A |
| **For Chunk** | Execution pin that fires for each chunk | Execution | N/A |
| **Chunk** | Output pin containing the current chunk of records | Struct | Array |
| **End** | Signals the completion of the node execution | Execution | N/A |

This node is particularly useful in scenarios where you need to process large CSV files efficiently, ensuring that memory usage remains manageable by processing the file in manageable chunks.