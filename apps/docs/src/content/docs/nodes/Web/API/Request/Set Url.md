---
title: SetUrlNode
description: Sets the URL of an HTTP request
---

## Purpose of the Node
This node is used to modify the URL of an existing HTTP request. It accepts an HTTP request and a new URL as inputs, and outputs the modified request with the new URL.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Request | The existing HTTP request whose URL needs to be set. | Struct | HttpRequest |
| Url | The new URL to be set for the HTTP request. | String | Normal |
| Request Out | The modified HTTP request with the updated URL. | Struct | HttpRequest |