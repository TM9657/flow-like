---
title: To Text Node
description: Converts the body of an HTTP response to text.
---

## Purpose of the Node
The To Text Node is designed to extract the body of an HTTP response and convert it to a text string. It is particularly useful in scenarios where you need to process the content of an API response directly as text.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to start the node's execution | Execution | - |
| Response | The HTTP response containing the body to be converted | Struct | HttpResponse |
| Text | The body of the response as text | String | - |
| End | Trigger called when the node execution is complete | Execution | - |
| Failed | Trigger called if the node execution fails | Execution | - |