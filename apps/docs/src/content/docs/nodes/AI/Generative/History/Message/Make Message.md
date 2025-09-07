---
title: Make History Message
description: Creates a ChatHistory struct with a Message.
---

## Purpose of the Node
This node generates a ChatHistory struct containing a Message based on the provided role and message type. The node dynamically adjusts the inputs based on the selected message type.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| role | The Role of the Message Author | String | Map |
| type | Message Type | String | Map |
| message | Constructed Message | Struct | Array |

Dynamic Pin Generation:
- If the message type is set to "Text", the "text" input pin is added and the "image" and "mime" pins are removed.
- If the message type is set to "Image", the "image" and "mime" input pins are added and the "text" pin is removed.