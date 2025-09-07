---
title: Last Content Extractor
description: This node extracts the content from the last message in a Response object, providing both the content string and a success flag.
---

## Purpose of the Node
The Last Content Extractor node is designed to extract the content from the last message of a Response object. It outputs both the extracted content and a success flag indicating whether the extraction was successful.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Response | Response to extract from | Struct | Response |
| Content | Content string from the last message | String | Normal |
| Success | Whether content was successfully extracted | Boolean | Normal |