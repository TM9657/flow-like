---
title: Set Bytes Body
description: A node to set the body of an HTTP request to a byte array.
---

## Purpose of the Node
This node is used to modify the body of an HTTP request to a byte array. It takes an HTTP request and a byte array as inputs, and outputs the modified HTTP request with the new body.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | The HTTP request to modify | Struct | HttpRequest |
| Body | The byte array to set as the request body | Byte | Array |
| End | The modified HTTP request with the new body | Struct | HttpRequest |