---
title: Set Struct Field
description: A node to set a field in a struct.
---

## Purpose of the Node
This node is used to modify a field within a struct. It takes a struct, a field name, and a value as inputs and outputs the modified struct.

## Pins
This node has four pins:

| Pin Name    | Pin Description                  | Pin Type | Value Type       |
|-------------|----------------------------------|----------|------------------|
| **Start**   | Initiate Execution               | Execution| N/A              |
| **End**     | Done with the Execution          | Execution| N/A              |
| **Struct In** | The struct to modify               | Struct   | HashMap          |
| **Field**   | The field name to modify           | String   | String           |
| **Value**   | The value to set for the field     | Generic  | Any Value Type   |
| **Struct Out** | The modified struct              | Struct   | HashMap          |