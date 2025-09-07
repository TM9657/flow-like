---
title: To Struct
description: Converts the body of an HTTP response to a JSON struct.
---

## Purpose of the Node
The To Struct node takes an HTTP response as input, extracts its body, and converts it to a JSON struct. It then outputs the JSON struct and an execution pin indicating successful completion.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to start the node | Execution | N/A |
| Response | The HTTP response to be converted | Struct | HttpResponse |
| Struct | The JSON struct representation of the response body | Struct | N/A |
| Failed | Triggered if the node fails | Execution | N/A |
| End | Triggered when the node is finished | Execution | N/A |