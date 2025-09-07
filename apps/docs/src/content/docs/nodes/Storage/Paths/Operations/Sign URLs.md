---
title: Sign URLs
description: Generates signed URLs for accessing files based on provided paths and HTTP method.
---

## Purpose of the Node
The Sign URLs node is designed to generate signed URLs for accessing files stored in a specific location. This is particularly useful for temporary access to private files, allowing users to download or access files securely for a specified duration.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node. | Exec | - |
| paths | Array of FlowPaths to sign. | Struct | Array |
| method | HTTP Method (GET, PUT, etc.) to use for signing. | String | Normal |
| expiration | Expiration time in seconds for the signed URLs. | Number | Normal |
| End | Indicates that the execution is complete. | Exec | - |
| signed_urls | The generated array of signed URLs. | String | Array |
| failed | Triggered if the signing process fails. | Exec | - |