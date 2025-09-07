---
title: InsertLocalDatabaseNode
description: A node for inserting a single value into a local database.
---

## Purpose of the Node
The `InsertLocalDatabaseNode` is used to insert a single value into a local database. It is faster than an upsert operation but may result in duplicate items if the value already exists in the database.

## Pins
This node has two input pins and two output pins.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Input trigger for the node to start executing. | Execution | - |
| **database** | Database connection reference. | Struct | `NodeDBConnection` |
| **value** | The value to be inserted into the database. | Struct | - |
| **End** | Output trigger indicating the operation has completed successfully. | Execution | - |
| **failed** | Output trigger indicating that the insertion failed. | Execution | - |