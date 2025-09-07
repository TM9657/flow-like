---
title: Get Header
description: Gets a header from an HTTP response.
---

## Purpose of the Node
This node retrieves the value of a specified header from an HTTP response. It outputs whether the header was found and its value.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Response | The HTTP response | Struct | HttpResponse |
| Header | The header to get | String | Normal |
| Found | True if the header was found | Boolean | Normal |
| Value | The value of the header | String | Normal |