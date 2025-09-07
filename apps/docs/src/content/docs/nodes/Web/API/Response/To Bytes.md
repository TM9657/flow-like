---
title: To Bytes
description: Converts an HTTP response to bytes.
---

## Purpose of the Node
This node extracts the body of an HTTP response and converts it to bytes. It is useful for processing the response data in binary form.

## Pins
The node has three input pins and two output pins. The `Start` pin triggers the node's execution, while the `response` pin accepts an HTTP response. The node outputs either `Start` upon successful execution or `failed` if an error occurs.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Triggers the node's execution | Execution | N/A |
| Response | The HTTP response | Struct | HttpResponse |
| End | Called when the node is finished | Execution | N/A |
| Bytes | The body of the response as bytes | Byte | Array |
| Failed | Called when the node fails | Execution | N/A |