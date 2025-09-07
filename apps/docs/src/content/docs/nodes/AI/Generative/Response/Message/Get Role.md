---
title: Get Role
description: Extracts the role from a message
---

## Purpose of the Node
This node extracts the role from a message, which is part of a response message structure. It is useful for scenarios where you need to process or conditionally handle different roles within your flow.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| message | Message to extract role from | Struct | ResponseMessage |
| role | Role string from the message | String | String |