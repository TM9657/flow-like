---
title: Is Success
description: Checks if the status code of an HTTP response is a success
---

## Purpose of the Node
This node evaluates an HTTP response and returns `true` if the status code indicates a success (200-299 range), and `false` otherwise.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| response | The HTTP response | Struct | HttpResponse |
| is_success | True if the status code is a success | Boolean | Normal |