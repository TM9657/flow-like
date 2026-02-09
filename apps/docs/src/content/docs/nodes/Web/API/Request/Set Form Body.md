---
title: Set Form Body
description: Sets the body of an HTTP request to form-encoded data.
---

## Purpose of the Node
The **Set Form Body** node encodes key-value fields as `application/x-www-form-urlencoded` and sets the body of an HTTP request. It can also add the content type header when missing.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| request | The HTTP request to modify. | Struct | HttpRequest |
| fields | Form fields to encode. | String | HashMap |
| set_content_type | Adds `application/x-www-form-urlencoded` when missing. | Boolean | Boolean |
| request_out | The modified HTTP request with the new body. | Struct | HttpRequest |
