---
title: Chunk Text Character
description: Chunks text into smaller pieces based on character capacity and overlap.
---

## Purpose of the Node
The Chunk Text Character node is designed to split a given text into smaller chunks based on specified character capacity and overlap. This is useful for efficient embedding processes where larger texts can be divided into manageable segments.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Struct | Execution |
| text | The string to be chunked | String | Normal |
| capacity | Character Capacity for each chunk | Number | Integer |
| overlap | Overlap between consecutive chunks | Number | Integer |
| markdown | Use Markdown Splitter? | Boolean | Integer |
| End | Execution completed | Struct | Execution |
| chunks | The resulting chunks | Array | String |
| failed | Execution failed | Struct | Execution |