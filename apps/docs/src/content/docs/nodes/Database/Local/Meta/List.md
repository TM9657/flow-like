---
title: List Local Database
description: Lists content from a local database with optional limit and offset.
---

## Purpose of the Node
The List Local Database node is designed to fetch a list of items from a locally stored database. It supports optional filtering via limit and offset parameters, and outputs the results as an array.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Input signal to initiate the node execution | Execution | None |
| Database | Reference to the database connection | Struct | NodeDBConnection |
| Limit | Maximum number of items to fetch | Integer | None |
| Offset | Starting point for the item fetch | Integer | None |
| End | Output signal indicating the completion of the node execution | Execution | None |
| Values | Array of fetched items | Struct | Array |