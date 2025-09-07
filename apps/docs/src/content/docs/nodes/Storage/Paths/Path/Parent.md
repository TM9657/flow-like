---
title: Parent Node
description: This node retrieves the parent path from a given `FlowPath`.
---

## Purpose of the Node
The Parent Node is designed to take a `FlowPath` as input and return its parent path. It provides an output indicating whether the execution was successful or if there was an error (e.g., on some systems the parent path might not be available).

## Pins

| Pin Name      | Pin Description                                               | Pin Type | Value Type   |
|:--------------|:--------------------------------------------------------------|:---------|:-------------|
| Start         | Initiates the execution of the node.                            | Struct   | Execution    |
| path          | The `FlowPath` from which the parent path needs to be derived. | Struct   | FlowPath     |
| End           | Marks the completion of the execution.                          | Struct   | Execution    |
| Parent Path   | The parent path derived from the input `FlowPath`.              | Struct   | FlowPath     |
| Failed        | Indicates if the operation to retrieve the parent path failed.  | Struct   | Execution    |