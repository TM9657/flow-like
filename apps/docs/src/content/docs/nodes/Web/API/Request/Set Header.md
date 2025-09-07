---
title: Set Header
description: A node that sets a header on an HTTP request.
---

## Purpose of the Node
The Set Header node is designed to modify the headers of an HTTP request before it is sent. It takes an HTTP request and two strings representing the header name and value, and then sets the specified header on the request.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Request | The HTTP request to modify | Struct | HttpRequest |
| Name | The name of the header to set | String | Normal |
| Value | The value of the header to set | String | Normal |
| Request Out | The modified HTTP request with the new header added | Struct | HttpRequest |