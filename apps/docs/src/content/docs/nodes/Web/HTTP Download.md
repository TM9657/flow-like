---
title: HttpDownloadNode
description: Downloads a file from a specified URL and saves it to a designated path.
---

## Purpose of the Node
This node is used to initiate an HTTP request to download a file from a specified URL and save it to a given path. It handles the execution flow based on the success or failure of the download operation.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the HTTP request | Execution | - |
| request | The HTTP request to perform | Struct | HttpRequest |
| flow_path | The path to save the file to | Struct | FlowPath |
| Success | Execution if the request succeeds | Execution | - |
| Error | Execution if the request fails | Execution | - |