---
title: Optimize Local Database
description: This node is designed to optimize and update a local database by removing unnecessary data or cleaning up old versions, depending on the user's preference.
---

## Purpose of the Node
This node is used to optimize and update a local database. It removes unnecessary data or cleans up old versions based on the user's preference to keep versions. If the optimization fails, it triggers an error output.

## Pins

| Pin Name    | Pin Description                                      | Pin Type | Value Type      |
|-------------|------------------------------------------------------|----------|-----------------|
| Start       | Input trigger to start the optimization process.     | Struct   | Execution       |
| Database    | Reference to the database connection to be optimized. | Struct   | NodeDBConnection|
| Keep Versions | Determines whether to keep old versions of data. | Boolean  | False           |
| End         | Triggered when the optimization is successful.     | Struct   | Execution       |
| Failed      | Triggered if the optimization fails.               | Struct   | Execution       |