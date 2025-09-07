---
title: Get Headers
description: Extracts headers from an HTTP request
---

## Purpose of the Node
The Get Headers node is used to retrieve and pass along the headers from an HTTP request. It accepts an HTTP request as input and outputs the headers as a JSON object.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Request** | The HTTP request from which headers will be extracted | Struct | HttpRequest |
| **Headers** | The headers extracted from the HTTP request | String | Map |