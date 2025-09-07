---
title: Get Database Schema Local
description: Retrieves the schema from a local database as a struct.
---

## Purpose of the Node
The "Get Database Schema Local" node retrieves the schema from a locally connected database and outputs it as a struct. This node is useful for dynamically understanding the structure of a database without hardcoding the schema details.

## Pins

| Pin Name   | Pin Description                                      | Pin Type | Value Type  |
|:-----------|:---------------------------------------------------|:---------|:------------|
| Start      | Input pin to trigger the node execution.           | Execution| None        |
| Database   | Database connection reference to fetch the schema from.| Struct   | NodeDBConnection|
| End        | Output pin to indicate that schema retrieval is done.| Execution| None        |
| Schema     | The retrieved local database schema.                 | Struct   | None        |