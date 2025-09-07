---
title: Get Content
description: Extracts the content from a message
---

## Purpose of the Node
This node extracts the content from a message, returning the content as a string and a boolean indicating whether the extraction was successful.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| message | Message to extract content from | Struct | ResponseMessage |
| content | Content string from the message | String | Normal |
| success | Whether content was successfully extracted | Boolean | Normal |