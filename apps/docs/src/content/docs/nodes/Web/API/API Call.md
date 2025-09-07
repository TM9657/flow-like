---
title: HTTP Fetch
description: Performs an HTTP request and handles the response.
---

## Purpose of the Node
This node is designed to initiate an HTTP request and handle both successful and error responses. It allows you to specify the request details and retrieve the response data.

## Pins
The node has three input pins and two output pins:

| Pin Name       | Pin Description                                     | Pin Type | Value Type     |
|----------------|-----------------------------------------------------|----------|----------------|
| Start          | Initiate the HTTP request.                          | Execution| Execution      |
| Request        | The HTTP request to perform.                        | Struct   | HttpRequest    |
| End            | Execution if the request fails.                     | Execution| Execution      |
| Response       | The HTTP response.                                  | Struct   | HttpResponse   |
| Success        | Execution if the request succeeds.                  | Execution| Execution      |