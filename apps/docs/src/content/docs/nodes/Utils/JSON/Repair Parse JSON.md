---
title: Repair Parse JSON
description: Attempts to repair and parse potentially malformed JSON
---

## Purpose of the Node
The Repair Parse JSON node is designed to handle and attempt to repair strings that may contain malformed JSON. It provides outputs to continue execution based on whether the parsing succeeds or fails.

## Pins
| Pin Name   | Pin Description                               | Pin Type | Value Type |
|------------|-----------------------------------------------|----------|------------|
| Start      | Initiate Execution                            | Struct   | Normal     |
| json_string| String containing potentially malformed JSON  | String   | Map        |
| End        | Execution continues if parsing succeeds       | Struct   | Normal     |
| result     | The parsed JSON structure                     | Struct   | Array      |
| failed     | Execution continues if parsing fails          | Struct   | Normal     |