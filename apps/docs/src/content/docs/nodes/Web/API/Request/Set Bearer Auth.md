---
title: Set Bearer Auth
description: Sets the Authorization header with a Bearer token.
---

## Purpose of the Node
The **Set Bearer Auth** node sets the `Authorization` header using a Bearer token, making it easy to add API authentication.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| request | The HTTP request to modify. | Struct | HttpRequest |
| token | The bearer token to use. | String | String |
| request_out | The modified HTTP request. | Struct | HttpRequest |
