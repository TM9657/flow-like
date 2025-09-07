---
title: Extract Content
description: Extracts the string content from a given message.
---

## Purpose of the Node
The Extract Content node is designed to extract the string content from a message, handling different types of message content such as plain text and multiple text segments. It is useful in scenarios where you need to process or display the text content from a history message.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| message | The input message from which the content will be extracted. | Struct | HistoryMessage |
| content | The extracted string content from the message. | String | Normal |