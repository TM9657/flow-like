---
title: Index Local Database
description: Builds an index on a specified column in a local database.
---

## Purpose of the Node
The Index Local Database node is designed to build an index on a specified column in a local database. This can improve query performance by allowing more efficient retrieval of data from the database.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger the execution of the node | Struct | Normal |
| Database | Database Connection Reference | Struct | Map |
| Column | Column to Index | String | Normal |
| Full-Text Search? | Is this index meant for full text search? | Boolean | Normal |
| End | Execution completes successfully | Struct | Normal |
| Failed | Execution fails to index the column | Struct | Normal |