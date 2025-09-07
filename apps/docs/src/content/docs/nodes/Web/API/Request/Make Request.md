---
title: Make Request
description: Creates a HTTP request based on the provided method and URL
---

## Purpose of the Node
The **Make Request** node is designed to generate an HTTP request. It takes the HTTP method and the URL as inputs and outputs a structured HTTP request object. This node is particularly useful in scenarios where you need to dynamically construct HTTP requests based on user inputs or other node outputs.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Method | Specifies the HTTP method for the request (GET, POST, PUT, DELETE, PATCH). Default value is "GET". | String | String |
| URL | The URL to which the HTTP request will be sent. | String | String |
| Request | The structured HTTP request object created from the provided method and URL. | Struct | HttpRequest |