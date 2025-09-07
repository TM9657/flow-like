---
title: DeleteLocalDatabaseNode
description: A node that deletes a local database based on an optional SQL filter.
---

## Purpose of the Node
The `DeleteLocalDatabaseNode` node is designed to delete a local database. It allows for an optional SQL filter to specify which records should be deleted.

## Pins
This node has four pins:

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | The input execution pin that triggers the node's logic. | Struct | Execution |
| **Database** | The database connection reference used to connect to the local database. | Struct | NodeDBConnection |
| **SQL Filter** | An optional SQL filter string to specify which records to delete. | String | Normal |
| **End** | The output execution pin that signifies the completion of the node's logic. | Struct | Execution |