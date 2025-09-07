---
title: Push Response
description: A node that pushes a chat response.
---

## Purpose of the Node
The Push Response node is used to push a chat response to a stream and signal the completion of the execution.

## Pins
| Pin Name   | Pin Description | Pin Type | Value Type |
|------------|-----------------|----------|------------|
| **Start**    | Initiate Execution | Execution | N/A        |
| **Response** | Chat Response | Struct | Response   |
| **End**      | Done with the Execution | Execution | N/A        |