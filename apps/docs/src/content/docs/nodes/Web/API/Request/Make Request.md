---
title: Make Request
description: Creates a HTTP request based on the provided method and URL
---

## Purpose of the Node
The **Make Request** node is designed to generate an HTTP request. It takes the HTTP method and the URL as inputs and outputs a structured HTTP request object. Optional headers can be attached for quick prototyping.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Method | Specifies the HTTP method for the request (GET, POST, PUT, DELETE, PATCH). Default value is "GET". | String | String |
| URL | The URL to which the HTTP request will be sent. | String | String |
| Headers | Optional request headers. | String | HashMap |
| Request | The structured HTTP request object created from the provided inputs. | Struct | HttpRequest |