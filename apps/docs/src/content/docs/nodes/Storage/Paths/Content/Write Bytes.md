---
title: WriteBytesNode
description: A node that writes bytes to a file using a specified path.
---

## Purpose of the Node
The WriteBytesNode is designed to write bytes to a file on the file system. It takes an input path and content to write, and then executes the write operation. If the operation fails, it triggers a "Failed" execution pin; otherwise, it triggers an "Output" execution pin.

## Pins
- **Path**: The flow path where the bytes will be written. This pin is required for the node to know where to store the content.
- **Content**: The actual content to write as bytes. This pin accepts an array of bytes.
- **Output**: Triggered when the write operation is successfully completed.
- **Failed**: Triggered if the write operation fails.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Path** | The flow path where the bytes will be written | Struct | FlowPath |
| **Content** | The content to write as bytes | Byte | Array |
| **Output** | Triggered when the write operation is successfully completed | Execution | - |
| **Failed** | Triggered if the write operation fails | Execution | - |