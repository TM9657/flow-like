---
title: Vector Search Local Database
description: Searches the local database based on a given vector.
---

## Purpose of the Node
The Vector Search Local Database node searches a local database using a specified vector. It supports optional SQL filters and result limits.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Initiates the node execution. | Exec | - |
| **Database** | A reference to the database connection. | Struct | NodeDBConnection |
| **Vector** | The vector used for searching the database. | Array | Float |
| **SQL Filter** | An optional SQL filter to refine search results. | String | - |
| **Limit** | The maximum number of results to return. | Integer | - |
| **Offset** | The starting point for the search results. | Integer | - |
| **End** | Signals the completion of node execution. | Exec | - |
| **Values** | The results of the search, returned as an array of found items. | Struct | Array |