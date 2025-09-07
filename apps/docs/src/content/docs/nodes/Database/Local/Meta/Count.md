---
title: CountLocalDatabase
description: A node to count items in a local database with an optional SQL filter.
---

## Purpose of the Node
The CountLocalDatabase node is designed to count the number of items in a local database. It accepts a database connection reference and an optional SQL filter. The node outputs the count of items found.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Triggers the execution of the node. | Execution | Normal |
| Database | Reference to the local database connection. | Struct | NodeDBConnection |
| SQL Filter | Optional SQL filter to refine the count query. | String | Map |
| End | Signals the completion of the node's execution. | Execution | Normal |
| Count | The count of items found in the database. | Integer | Normal |