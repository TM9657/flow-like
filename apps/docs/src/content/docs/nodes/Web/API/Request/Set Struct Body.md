---
title: Set Struct Body
description: A node that sets the body of an HTTP request to a JSON struct.
---

## Purpose of the Node
This node is used to set the body of an HTTP request to a JSON struct. It takes an HTTP request and a struct as input, and outputs the modified HTTP request with the new body.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | The input HTTP request and the body to be set | Struct | HttpRequest |
| End | The output HTTP request with the updated body | Struct | HttpRequest |