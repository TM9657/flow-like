---
title: Delete Node
description: Deletes a file or directory.
---

## Purpose of the Node
The Delete Node is used to delete a specified file or directory from a storage path. It provides options for recursive deletion, allowing directories to be deleted along with their contents.

## Pins

| Pin Name     | Pin Description                            | Pin Type | Value Type |
|--------------|--------------------------------------------|----------|------------|
| **Start**    | Initiates the execution of the node.       | Execution | -          |
| **Path**     | The path to the file or directory to delete.| Struct    | FlowPath   |
| **Recursive**| Whether to delete directories recursively. | Boolean   | Boolean    |
| **End**      | Marks the end of successful execution.   | Execution | -          |
| **End (Failure)** | Marks the end of failed execution.       | Execution | -          |