---
title: Set Content-Type
description: Sets the Content-Type header of an HTTP request.
---

## Purpose of the Node
The **Set Content-Type** node sets the `Content-Type` header for an HTTP request.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| request | The HTTP request to modify. | Struct | HttpRequest |
| content_type | The content type value to set. | String | String |
| request_out | The modified HTTP request. | Struct | HttpRequest |
