---
title: Copy
description: Copies a file from one location to another
---

## Purpose of the Node
This node is used to copy a file from a specified source path to a specified destination path. It handles both the direct copy of files and the efficient transfer of large files using multipart uploads.

## Pins
This node has three input pins and two output pins:

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the copy operation | Exec | - |
| From | Specifies the source path of the file to be copied | Struct | FlowPath |
| To | Specifies the destination path where the file should be copied | Struct | FlowPath |
| Success | Activates when the file copy is successful | Exec | - |
| Failure | Activates when the file copy fails | Exec | - |