---
title: Create Local Database
description: Creates a local database connection
---

## Purpose of the Node
The Create Local Database node is used to open a local database connection based on the provided table name. It checks if the database is already cached; if not, it builds and caches the database connection.

## Pins
| Pin Name   | Pin Description            | Pin Type | Value Type  |
|------------|----------------------------|----------|-------------|
| Start      | Input signal to start the process | Execution | N/A         |
| Table Name | Name of the Table to connect | String    | N/A         |
| End        | Output signal indicating completion | Execution | N/A         |
| Database   | Database Connection Reference | Struct    | NodeDBConnection |