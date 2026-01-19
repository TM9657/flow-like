---
title: Set Accept
description: Sets the Accept header of an HTTP request.
---

## Purpose of the Node
The **Set Accept** node sets the `Accept` header for an HTTP request.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| request | The HTTP request to modify. | Struct | HttpRequest |
| accept | The accept header value to set. | String | String |
| request_out | The modified HTTP request. | Struct | HttpRequest |
