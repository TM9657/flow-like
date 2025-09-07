---
title: Full-Text Search Local Database
description: This node performs a full-text search on a local database using a provided vector and text search term.
---

## Purpose of the Node
This node is designed to search a local database for items that match a full-text search term. It can also include an optional SQL filter and allows for limiting and offsetting the results.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | The input signal to initiate the search operation | Execution | - |
| Database | Reference to the database connection | Struct | NodeDBConnection |
| Search Term | The full-text search term to look for in the database | String | - |
| SQL Filter | An optional SQL filter to apply to the search results | String | - |
| Limit | The maximum number of results to return | Integer | - |
| Offset | The number of results to skip before starting to return results | Integer | - |
| End | The output signal indicating the completion of the search operation | Execution | - |
| Values | The results of the search, returned as an array | Struct | Array |