---
title: HTML to Markdown
description: Converts HTML to Markdown, optionally removing specified tags.
---

## Purpose of the Node
This node is designed to convert HTML content into Markdown format, offering the flexibility to specify tags that should be skipped during the conversion process.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | None |
| Html | Html to Parse | String | None |
| Tags | Tags to skip | String | Array |
| End | Finished Parsing | Execution | None |
| Markdown | The parsed Markdown | String | None |