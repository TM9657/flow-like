---
title: Streaming HTTP Fetch
description: Performs an HTTP request and streams the response.
---

## Purpose of the Node
The Streaming HTTP Fetch node is designed to initiate an HTTP request and stream the response to connected nodes. It handles both success and error scenarios, providing appropriate execution pins to indicate the state of the request.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate the HTTP request | Execution | None |
| Request | The HTTP request to perform | Struct | HttpRequest |
| On Stream | Intermediate Result | Execution | None |
| Stream Response | The HTTP response | Byte | Array |
| Success | Execution if the request succeeds | Execution | None |
| Response | The HTTP response | Struct | HttpResponse |
| Error | Execution if the request fails | Execution | None |