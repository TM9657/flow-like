---
title: Sign URL
description: Generates a signed URL for accessing a file.
---

## Purpose of the Node
This node is used to generate a signed URL for accessing a file, which can be useful for temporary access to files stored in a secure location.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node | Execution | N/A |
| Path | The FlowPath for the file to be signed | Struct | FlowPath |
| Method | The HTTP method to be used for the signed URL (GET, PUT, POST, DELETE, HEAD) | String | GET |
| Expiration (seconds) | The expiration time in seconds for the signed URL | Integer | 3600 |
| End | Marks the completion of the execution | Execution | N/A |
| Signed URL | The generated signed URL | String | N/A |
| Failed | Triggered if the signing process fails | Execution | N/A |