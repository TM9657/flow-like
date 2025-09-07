---
title: Upsert Local Database
description: Inserts or updates data in a local database.
---

## Purpose of the Node
This node is designed to either insert new data or update existing data in a local database. It accepts a database connection reference, an ID column, and a value to be upserted. The node provides feedback through execution pins to indicate success or failure.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger the node execution | Execution | - |
| Database | Database Connection Reference | Struct | `NodeDBConnection` |
| ID Column | The ID Column | String | - |
| Value | Value to Insert | Struct | - |
| End | Indicates successful execution | Execution | - |
| Failed | Triggered if the Upsert failed | Execution | - |