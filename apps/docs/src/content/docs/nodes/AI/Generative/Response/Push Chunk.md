---
title: Push Chunk Node
description: This node adds a response chunk to a Response.
---

## Purpose of the Node
This node is designed to add a new response chunk to an existing `Response` object. It takes an input `Response` and a `ResponseChunk`, updates the `Response` by adding the chunk, and outputs the updated `Response`.

## Pins
| Pin Name   | Pin Description                                      | Pin Type | Value Type |
|:----------:|:----------------------------------------------------:|:--------:|:----------:|
| Start      | Initiate Execution                                     | Exec     | N/A        |
| Response   | Response to update                                   | Struct   | Response   |
| Chunk      | Response chunk to add                                | Struct   | ResponseChunk |
| End        | Done with the Execution                                | Exec     | N/A        |
| ResponseOut | Updated Response                                     | Struct   | Response   |