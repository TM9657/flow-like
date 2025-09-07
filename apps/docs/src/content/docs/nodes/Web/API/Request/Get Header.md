---
title: Get Header
description: Retrieves the value of a specified header from an HTTP request.
---

## Purpose of the Node
This node is designed to extract the value of a specified header from an incoming HTTP request. It takes an HTTP request object and a header name as inputs, and outputs whether the header was found and its corresponding value.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Request | The HTTP request from which to extract the header. | Struct | HttpRequest |
| Header | The name of the header to extract. | String | String |
| Found | Indicates whether the specified header was found in the request. | Boolean | Boolean |
| Value | The value of the specified header. If the header is not found, an empty string is returned. | String | String |