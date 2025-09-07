---
title: Set String Body
description: Sets the body of an HTTP request to a string value.
---

## Purpose of the Node
Sets the body of an HTTP request to a specified string value, allowing for dynamic content manipulation in web API interactions.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| request | The HTTP request to modify. | Struct | HttpRequest |
| body | The string body content to set. | String | Normal |
| request_out | The modified HTTP request with the new body. | Struct | HttpRequest |