---
title: Purge Local Database
description: This node purges a local database by clearing its contents.
---

## Purpose of the Node
The Purge Local Database node is used to clear all data from a specified local database. It takes a database connection reference as input and executes the purge operation.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | The start trigger for the node. | Struct | Normal |
| Database | Reference to the local database connection to be purged. | Struct | NodeDBConnection |
| End | The end trigger indicating the completion of the purge operation. | Struct | Normal |