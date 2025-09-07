---
title: Get Status Code
description: A node that extracts the status code from an HTTP response.
---

## Purpose of the Node
The Get Status Code node retrieves the status code from an HTTP response and outputs it as an integer. This is useful for conditionally executing other nodes based on the response status.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Response | The HTTP response from which the status code is extracted | Struct | HttpResponse |
| Status Code | The status code of the HTTP response | Integer | Normal |