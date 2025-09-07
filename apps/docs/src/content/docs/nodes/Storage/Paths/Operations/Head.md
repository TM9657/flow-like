---
title: Head
description: A node to get the metadata of a file.
---

## Purpose of the Node
The Head node retrieves the metadata (ETag, Last Modified, Size, and Version) of a file specified by a given flow path. It acts as a starting point for operations on file metadata.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Execution |
| Path | FlowPath | Struct | FlowPath |
| ETag | Etag of the file | String | String |
| Last Modified | Last Modified timestamp of the file | Date | Date |
| Size | Size of the file in bytes | Integer | Integer |
| Version | Version of the file | String | String |
| Failed | Signal execution failure | Execution | Execution |
| End | Signal completion of execution | Execution | Execution |