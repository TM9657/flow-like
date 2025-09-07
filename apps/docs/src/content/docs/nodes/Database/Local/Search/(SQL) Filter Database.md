---
title: Filter Local Database
description: A node to filter data from a local database using SQL queries.
---

## Purpose of the Node
The Filter Local Database node allows you to filter data from a local database using SQL queries. It takes a database connection reference, an optional SQL filter, a limit, and an offset as inputs, and outputs the filtered results and an execution completion signal.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger input for the node to start processing. | Execution | - |
| Database | Database Connection Reference. | Struct | NodeDBConnection |
| SQL Filter | Optional SQL Filter to apply to the database. | String | - |
| Limit | Limit the number of results returned. | Integer | - |
| Offset | Offset the results by a specified number of records. | Integer | - |
| End | Output signal indicating the completion of the node execution. | Execution | - |
| Values | Found items after filtering. | Array | - |