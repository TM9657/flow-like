---
title: Last Message
description: Extracts the last message from a Response
---

## Purpose of the Node
The Last Message node extracts the last message from a given Response object and provides it as an output, along with a success flag indicating whether the extraction was successful.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Response | Response to extract from | Struct | Response |
| message | Last message from the response | Struct | ResponseMessage |
| success | Whether a message was successfully extracted | Boolean | Normal |