---
title: Get Headers
description: A node to extract headers from an HTTP response.
---

## Purpose of the Node
The `Get Headers` node retrieves all headers from a given HTTP response and outputs them as a JSON object.

## Pins
- **response**: The HTTP response from which headers are to be extracted. It accepts a structured value of type `HttpResponse`.
- **headers**: The extracted headers from the HTTP response, output as a string containing a JSON object.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| response | The HTTP response from which headers are to be extracted. | Struct | HttpResponse |
| headers | The extracted headers from the HTTP response, output as a string containing a JSON object. | String | HashMap |