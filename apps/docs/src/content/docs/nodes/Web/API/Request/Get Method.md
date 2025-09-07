---
title: Get Method
description: This node retrieves the HTTP method from a given request.
---

## Purpose of the Node
This node is designed to extract the HTTP method from an incoming HTTP request and output it as a string.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| request | The HTTP request from which to extract the method | Struct | HttpRequest |
| method | The HTTP method of the request | String | Normal |